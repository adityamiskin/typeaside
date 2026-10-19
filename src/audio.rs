mod lid;

use crate::config::AppConfig;
use anyhow::{Context, Result, anyhow, bail};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig};
use serde::Serialize;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU32, Ordering},
};

pub struct AudioRecorder {
    stream: Stream,
    buffer: Arc<Mutex<Vec<f32>>>,
    level: Arc<AtomicU32>,
    paused: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
    sample_rate: u32,
    realtime: Option<Arc<Mutex<Realtime>>>,
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
        if selected.is_none()
            && let Some(external) = lid::external_input(&host, &device)
            && let Ok(description) = external.description()
            && let Ok(recorder) = Self::open_device(&external, retain, sink.clone())
        {
            eprintln!("typeaside: lid closed, using {}", description.name());
            return Ok(recorder);
        }
        Self::open_device(&device, retain, sink)
    }

    fn open_device(
        device: &cpal::Device,
        retain: bool,
        sink: Option<crate::stream::AudioSink>,
    ) -> Result<Self> {
        let supported = device.default_input_config()?;
        let stream_config = supported.config();
        let realtime = sink.map(|sink| {
            Arc::new(Mutex::new(Realtime {
                sink,
                emitted: 0,
                pending: Vec::new(),
            }))
        });
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
            stream,
            buffer,
            level,
            paused,
            error,
            sample_rate: stream_config.sample_rate,
            realtime,
        })
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

    pub fn stop(self) -> Result<Vec<f32>> {
        drop(self.stream);
        if let Some(error) = self
            .error
            .lock()
            .map_err(|_| anyhow!("audio error lock poisoned"))?
            .take()
        {
            bail!("audio capture failed: {error}");
        }
        let data = self
            .buffer
            .lock()
            .map_err(|_| anyhow!("audio buffer poisoned"))?;
        if let Some(realtime) = self.realtime
            && let Ok(mut realtime) = realtime.lock()
        {
            realtime.push(&data, self.sample_rate, true);
        }
        Ok(resample(&data, self.sample_rate))
    }
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
    realtime: Option<Arc<Mutex<Realtime>>>,
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
                for frame in samples.chunks_exact(channels).take(if retain {
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
                    if retain {
                        buffer.push(mono);
                    }
                    power += smoothing * (mono * mono - power);
                }
                if let Some(realtime) = &realtime
                    && let Ok(mut realtime) = realtime.lock()
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
        while self.pending.len() >= 1600 {
            self.sink.send(&self.pending[..1600]);
            self.pending.drain(..1600);
        }
        if flush && !self.pending.is_empty() {
            self.sink.send(&self.pending);
            self.pending.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_limit_bounds_each_callback_at_ten_minutes() {
        for rate in [8_000, 16_000, 44_100, 48_000, 96_000] {
            let limit = rate as usize * 600;
            assert_eq!(remaining_samples(0, rate), limit);
            assert_eq!(remaining_samples(limit - 1, rate), 1);
            assert_eq!(remaining_samples(limit, rate), 0);
            assert_eq!(remaining_samples(limit + 1024, rate), 0);
        }
    }

    #[test]
    fn preferred_input_uses_first_available_preference() {
        let available = ["Built-in", "USB", "Dock"].map(str::to_owned);
        for (preferences, expected) in [
            (vec!["Dock", "USB"], Some("Dock")),
            (vec!["Missing", "USB", "Dock"], Some("USB")),
            (vec!["Missing"], None),
            (vec![], None),
        ] {
            let preferences = preferences
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            assert_eq!(preferred_input(&available, &preferences), expected);
        }
        assert_eq!(preferred_input(&[], &available), None);
    }

    #[test]
    fn streamed_resampling_matches_final_buffer() {
        for rate in [8_000, 16_000, 44_100, 48_000, 96_000] {
            let samples: Vec<_> = (0..rate + 137)
                .map(|i| (i as f32 * 0.017).sin() * 0.8)
                .collect();
            let expected = resample(&samples, rate);
            for chunk in [1, 137, 441, 1024, 4801] {
                let mut streamed = Vec::new();
                let mut available = Vec::new();
                for input in samples.chunks(chunk) {
                    available.extend_from_slice(input);
                    streamed.extend(resample_from(&available, rate, streamed.len(), false));
                }
                streamed.extend(resample_from(&available, rate, streamed.len(), true));
                assert_eq!(streamed, expected, "rate={rate} chunk={chunk}");
            }
        }
    }
}
