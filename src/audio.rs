mod lid;

use crate::config::AppConfig;
use anyhow::{Context, Result, anyhow, bail};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU32, Ordering},
};

pub struct AudioRecorder {
    stream: Option<Stream>,
    buffer: Arc<Mutex<Vec<f32>>>,
    level: Arc<AtomicU32>,
    paused: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
    sample_rate: u32,
    realtime: Arc<Mutex<Option<Realtime>>>,
    ready: Arc<Mutex<Option<Ready>>>,
    device_id: String,
}

const RING_SAMPLES: usize = 16_000;
const PRE_ROLL_SAMPLES: usize = 7_200;

#[derive(Default)]
struct PreRoll {
    samples: VecDeque<f32>,
}

impl PreRoll {
    fn extend(&mut self, samples: impl IntoIterator<Item = f32>) {
        for sample in samples {
            if self.samples.len() == RING_SAMPLES {
                self.samples.pop_front();
            }
            self.samples.push_back(sample);
        }
    }

    fn take(&mut self) -> Vec<f32> {
        let skip = self.samples.len().saturating_sub(PRE_ROLL_SAMPLES);
        self.samples.drain(..skip);
        self.samples.drain(..).collect()
    }
}

#[derive(Default)]
struct Ready {
    ring: PreRoll,
    pending: Vec<f32>,
    preroll: Vec<f32>,
    recording: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct InputDeviceInfo {
    pub id: usize,
    pub name: String,
    pub is_default: bool,
}

pub fn list_input_devices() -> Result<Vec<InputDeviceInfo>> {
    let host = cpal::default_host();
    let default = host
        .default_input_device()
        .and_then(|device| device.description().ok())
        .map(|d| d.name().to_owned());
    host.input_devices()?
        .enumerate()
        .map(|(id, device)| {
            let name = device.description()?.name().to_owned();
            Ok(InputDeviceInfo {
                id,
                is_default: default.as_ref() == Some(&name),
                name,
            })
        })
        .collect()
}

fn preferred_input<'a>(available: &[String], preferences: &'a [String]) -> Option<&'a str> {
    preferences
        .iter()
        .find(|name| available.contains(name))
        .map(String::as_str)
}

impl AudioRecorder {
    pub fn start(config: &AppConfig) -> Result<Self> {
        Self::open(config, true, None)
    }

    pub fn monitor(config: &AppConfig) -> Result<Self> {
        Self::open(config, false, None)
    }

    pub fn start_streaming(config: &AppConfig, sink: crate::stream::AudioSink) -> Result<Self> {
        Self::open(config, true, Some(sink))
    }

    fn open(
        config: &AppConfig,
        retain: bool,
        sink: Option<crate::stream::AudioSink>,
    ) -> Result<Self> {
        let (device, external) = Self::devices(config)?;
        if let Some(external) = external
            && let Ok(description) = external.description()
            && let Ok(recorder) = Self::open_device(&external, retain, sink.clone())
        {
            eprintln!("typeaside: lid closed, using {}", description.name());
            return Ok(recorder);
        }
        Self::open_device(&device, retain, sink)
    }

    fn devices(config: &AppConfig) -> Result<(cpal::Device, Option<cpal::Device>)> {
        let host = cpal::default_host();
        let devices: Vec<_> = host
            .input_devices()?
            .filter_map(|device| {
                let name = device.description().ok()?.name().to_owned();
                Some((name, device))
            })
            .collect();
        let names: Vec<_> = devices.iter().map(|(name, _)| name.clone()).collect();
        let selected = preferred_input(&names, &config.input_devices);
        let device = selected
            .and_then(|name| devices.iter().find(|(candidate, _)| candidate == name))
            .map(|(_, device)| device.clone())
            .or_else(|| host.default_input_device())
            .context("no input device available")?;
        let external = selected
            .is_none()
            .then(|| lid::external_input(&host, &device))
            .flatten();
        Ok((device, external))
    }

    fn open_device(
        device: &cpal::Device,
        retain: bool,
        sink: Option<crate::stream::AudioSink>,
    ) -> Result<Self> {
        let supported = device.default_input_config()?;
        let stream_config = supported.config();
        let realtime = Arc::new(Mutex::new(sink.map(|sink| Realtime {
            sink,
            emitted: 0,
            pending: Vec::new(),
        })));
        let ready = Arc::new(Mutex::new(None));
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let level = Arc::new(AtomicU32::new(0));
        let paused = Arc::new(AtomicBool::new(false));
        let error = Arc::new(Mutex::new(None));
        macro_rules! capture_format {
            ($sample:ty) => {
                capture::<$sample>(
                    device,
                    &stream_config,
                    buffer.clone(),
                    level.clone(),
                    paused.clone(),
                    error.clone(),
                    retain,
                    realtime.clone(),
                    ready.clone(),
                )
            };
        }
        let stream = match supported.sample_format() {
            SampleFormat::I8 => capture_format!(i8),
            SampleFormat::I16 => capture_format!(i16),
            SampleFormat::I24 => capture_format!(cpal::I24),
            SampleFormat::I32 => capture_format!(i32),
            SampleFormat::I64 => capture_format!(i64),
            SampleFormat::U8 => capture_format!(u8),
            SampleFormat::U16 => capture_format!(u16),
            SampleFormat::U24 => capture_format!(cpal::U24),
            SampleFormat::U32 => capture_format!(u32),
            SampleFormat::U64 => capture_format!(u64),
            SampleFormat::F32 => capture_format!(f32),
            SampleFormat::F64 => capture_format!(f64),
            format => bail!("unsupported input format: {format:?}"),
        }?;
        stream.play()?;
        Ok(Self {
            stream: Some(stream),
            buffer,
            level,
            paused,
            error,
            sample_rate: stream_config.sample_rate,
            realtime,
            ready,
            device_id: device_id(device),
        })
    }

    pub(crate) fn ready(config: &AppConfig) -> Result<Self> {
        let recorder = Self::open(config, true, None)?;
        *recorder
            .ready
            .lock()
            .map_err(|_| anyhow!("audio buffer poisoned"))? = Some(Ready::default());
        Ok(recorder)
    }

    pub(crate) fn matches_device(&self, config: &AppConfig) -> bool {
        Self::devices(config)
            .map(|(device, external)| device_id(&external.unwrap_or(device)))
            .is_ok_and(|id| id == self.device_id)
    }

    pub(crate) fn begin(&self, sink: Option<crate::stream::AudioSink>) -> Result<()> {
        let mut buffer = self
            .buffer
            .lock()
            .map_err(|_| anyhow!("audio buffer poisoned"))?;
        let mut state = self
            .ready
            .lock()
            .map_err(|_| anyhow!("audio buffer poisoned"))?;
        let ready = state.as_mut().context("microphone not ready")?;
        buffer.clear();
        ready.preroll = ready.ring.take();
        ready.pending.clear();
        ready.recording = true;
        let mut realtime = self
            .realtime
            .lock()
            .map_err(|_| anyhow!("audio buffer poisoned"))?;
        *realtime = sink.map(|sink| {
            for chunk in ready.preroll.chunks(sink.block) {
                sink.send(chunk);
            }
            Realtime {
                sink,
                emitted: 0,
                pending: Vec::new(),
            }
        });
        if std::env::var_os("TYPEASIDE_TRACE").is_some_and(|v| v == "1") {
            eprintln!(
                "typeaside: pre-roll prepended samples={}",
                ready.preroll.len()
            );
        }
        Ok(())
    }

    pub(crate) fn preroll_samples(&self) -> usize {
        self.ready
            .lock()
            .ok()
            .and_then(|state| state.as_ref().map(|ready| ready.preroll.len()))
            .unwrap_or(0)
    }

    pub(crate) fn take_error(&self) -> Option<String> {
        self.error.lock().ok()?.take()
    }

    pub fn limit_reached(&self) -> bool {
        self.buffer
            .lock()
            .is_ok_and(|buffer| remaining_samples(buffer.len(), self.sample_rate) == 0)
    }

    pub fn toggle_paused(&self) -> bool {
        !self.paused.fetch_xor(true, Ordering::Relaxed)
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn current_level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed))
    }

    pub(crate) fn sample_count(&self) -> usize {
        self.buffer.lock().map_or(0, |buffer| {
            (buffer.len() as f64 * 16_000.0 / f64::from(self.sample_rate)).floor() as usize
        })
    }

    pub fn stop(mut self, keep_ready: bool) -> Result<(Vec<f32>, Option<Self>)> {
        let keep_ready = keep_ready && self.ready.lock().is_ok_and(|state| state.is_some());
        if !keep_ready {
            self.stream = None;
        }
        self.paused.store(false, Ordering::Relaxed);
        if let Some(error) = self.take_error() {
            bail!("audio capture failed: {error}");
        }
        let audio = {
            let mut data = self
                .buffer
                .lock()
                .map_err(|_| anyhow!("audio buffer poisoned"))?;
            let mut state = self
                .ready
                .lock()
                .map_err(|_| anyhow!("audio buffer poisoned"))?;
            let mut realtime = self
                .realtime
                .lock()
                .map_err(|_| anyhow!("audio buffer poisoned"))?;
            if let Some(mut realtime) = realtime.take() {
                realtime.push(&data, self.sample_rate, true);
            }
            let mut audio = state
                .as_mut()
                .map(std::mem::take)
                .unwrap_or_default()
                .preroll;
            audio.extend(resample(&data, self.sample_rate));
            data.clear();
            audio
        };
        Ok((audio, keep_ready.then_some(self)))
    }
}

fn device_id(device: &cpal::Device) -> String {
    device.id().map(|id| id.to_string()).unwrap_or_default()
}

fn remaining_samples(length: usize, rate: u32) -> usize {
    (rate as usize * 600).saturating_sub(length)
}

#[allow(clippy::too_many_arguments)]
fn capture<T: SizedSample + Sample>(
    device: &cpal::Device,
    config: &StreamConfig,
    buffer: Arc<Mutex<Vec<f32>>>,
    level: Arc<AtomicU32>,
    paused: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
    retain: bool,
    realtime: Arc<Mutex<Option<Realtime>>>,
    ready: Arc<Mutex<Option<Ready>>>,
) -> Result<Stream>
where
    f32: FromSample<T>,
{
    let rate = config.sample_rate;
    let channels = usize::from(config.channels);
    let smoothing = 1.0 - (-1.0 / (config.sample_rate as f32 * 0.08)).exp();
    let mut power = 0.0;
    Ok(device.build_input_stream(
        config,
        move |samples: &[T], _| {
            if let Ok(mut buffer) = buffer.lock() {
                if paused.load(Ordering::Relaxed) {
                    power = 0.;
                    level.store(0, Ordering::Relaxed);
                    return;
                }
                let mut state = match ready.lock() {
                    Ok(state) => state,
                    Err(_) => return,
                };
                let idle = state.as_ref().is_some_and(|ready| !ready.recording);
                for frame in samples.chunks_exact(channels).take(if retain && !idle {
                    remaining_samples(buffer.len(), rate)
                } else {
                    usize::MAX
                }) {
                    let mono = frame
                        .iter()
                        .map(|sample| f32::from_sample(*sample))
                        .sum::<f32>()
                        / channels as f32;
                    let mono = if mono.is_finite() {
                        mono.clamp(-1.0, 1.0)
                    } else {
                        0.0
                    };
                    if idle {
                        if let Some(ready) = state.as_mut() {
                            ready.pending.push(mono);
                        }
                    } else if retain {
                        buffer.push(mono);
                    }
                    power += smoothing * (mono * mono - power);
                }
                if idle && let Some(ready) = state.as_mut() {
                    let block = rate as usize / 100;
                    let length = ready.pending.len() / block * block;
                    ready.ring.extend(resample(&ready.pending[..length], rate));
                    ready.pending.drain(..length);
                }
                if let Ok(mut realtime) = realtime.lock()
                    && let Some(realtime) = realtime.as_mut()
                {
                    realtime.push(&buffer, rate, false);
                }
                let perceptual =
                    ((20.0 * power.max(1e-10).sqrt().log10() + 60.0) / 60.0).clamp(0.0, 1.0);
                level.store(perceptual.to_bits(), Ordering::Relaxed);
            }
        },
        move |err| {
            if let Ok(mut error) = error.lock() {
                *error = Some(err.to_string());
            }
        },
        None,
    )?)
}

fn resample(samples: &[f32], rate: u32) -> Vec<f32> {
    resample_from(samples, rate, 0, true)
}

fn resample_from(samples: &[f32], rate: u32, offset: usize, flush: bool) -> Vec<f32> {
    let width = rate as f64 / 16_000.0;
    let mut length = (samples.len() as f64 / width).floor() as usize;
    // Floating-point window boundaries can extend just past an integer input position.
    if !flush && (length as f64 * width).ceil() as usize > samples.len() {
        length = length.saturating_sub(1);
    }
    (offset..length)
        .map(|i| {
            let start = i as f64 * width;
            let end = (i + 1) as f64 * width;
            let mut sum = 0.0;
            for (j, sample) in samples
                .iter()
                .enumerate()
                .take((end.ceil() as usize).min(samples.len()))
                .skip(start.floor() as usize)
            {
                let overlap = (end.min((j + 1) as f64) - start.max(j as f64)).max(0.0);
                sum += f64::from(*sample) * overlap;
            }
            (sum / width) as f32
        })
        .collect()
}

struct Realtime {
    sink: crate::stream::AudioSink,
    emitted: usize,
    pending: Vec<f32>,
}

impl Realtime {
    fn push(&mut self, samples: &[f32], rate: u32, flush: bool) {
        let next = resample_from(samples, rate, self.emitted, flush);
        self.emitted += next.len();
        self.pending.extend(next);
        let block = self.sink.block;
        while self.pending.len() >= block {
            self.sink.send(&self.pending[..block]);
            self.pending.drain(..block);
        }
        if flush && !self.pending.is_empty() {
            self.sink.send(&self.pending);
            self.pending.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ready_ring_keeps_only_latest_second_and_prepends_latest_450ms() {
        let mut ring = super::PreRoll::default();
        ring.extend((0..20_000).map(|i| i as f32));
        assert_eq!(ring.samples.len(), 16_000);
        let clip = ring.take();
        assert_eq!(clip.len(), 7200);
        assert_eq!(clip[0], 12_800.);
        assert_eq!(clip[7199], 19_999.);
        assert!(ring.take().is_empty());
        ring.extend([1., 2., 3.]);
        assert_eq!(ring.take(), [1., 2., 3.]);
    }

    #[test]
    fn resampling_preserves_release_tail_sample_count() {
        for rate in [16_000, 44_100, 48_000] {
            let held = vec![0.1; rate as usize];
            let mut captured = held.clone();
            captured.extend(vec![0.2; rate as usize / 5]);
            let held_samples = super::resample(&held, rate).len();
            let audio = super::resample(&captured, rate);
            assert_eq!(held_samples, 16_000);
            assert_eq!(audio.len(), 19_200);
            assert_eq!(audio.len() - held_samples, 3200);
        }
    }
}
