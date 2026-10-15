use anyhow::{Context, Result};
use cpal::{
    FromSample, SampleFormat, SizedSample,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use std::{f32::consts::TAU, sync::mpsc, time::Duration};

#[derive(Clone, Copy, Debug)]
pub enum Cue {
    Start,
    Stop,
    Error,
}

impl Cue {
    pub const ALL: [Self; 3] = [Self::Start, Self::Stop, Self::Error];
}

struct Thunk {
    at: f32,
    pitch: f32,
    gain: f32,
}

/// Mono samples for `cue` at `rate` Hz: muted, woody knocks rather than beeps.
pub fn render(cue: Cue, rate: u32) -> Vec<f32> {
    let thunks: &[Thunk] = match cue {
        Cue::Start => &[
            Thunk {
                at: 0.,
                pitch: 220.,
                gain: 1.,
            },
            Thunk {
                at: 0.004,
                pitch: 330.,
                gain: 0.35,
            },
        ],
        Cue::Stop => &[Thunk {
            at: 0.,
            pitch: 165.,
            gain: 0.9,
        }],
        Cue::Error => &[
            Thunk {
                at: 0.,
                pitch: 131.,
                gain: 0.8,
            },
            Thunk {
                at: 0.11,
                pitch: 123.,
                gain: 0.7,
            },
        ],
    };
    let rate = rate as f32;
    let length = thunks.iter().map(|thunk| thunk.at).fold(0., f32::max) + 0.32;
    let mut samples = vec![0f32; (length * rate) as usize];
    for thunk in thunks {
        add_thunk(&mut samples, rate, thunk);
    }
    let fade = 0.04 * rate;
    let total = samples.len();
    for (index, sample) in samples.iter_mut().enumerate() {
        *sample = (*sample * 1.4).tanh() * 0.32 * ((total - index) as f32 / fade).min(1.);
    }
    samples
}

fn add_thunk(samples: &mut [f32], rate: f32, thunk: &Thunk) {
    let start = (thunk.at * rate) as usize;
    let mut phase = 0f32;
    let mut noise_seed = 0x9e37_79b9u32;
    let mut noise = 0f32;
    for (n, sample) in samples.iter_mut().skip(start).enumerate() {
        let t = n as f32 / rate;
        // The pitch falls into place like a struck mallet, which reads as "thunk" not "beep".
        phase += TAU * thunk.pitch * (1. + 0.5 * (-t / 0.01).exp()) / rate;
        let attack = (t / 0.0015).min(1.);
        let body = phase.sin() * (-t / 0.06).exp();
        let wood = (phase * 2.76).sin() * (-t / 0.016).exp() * 0.3;
        noise_seed ^= noise_seed << 13;
        noise_seed ^= noise_seed >> 17;
        noise_seed ^= noise_seed << 5;
        noise += (noise_seed as f32 / u32::MAX as f32 * 2. - 1. - noise) * 0.25;
        let click = noise * (-t / 0.002).exp() * 0.5;
        *sample += (body + wood + click) * attack * thunk.gain;
    }
}

fn spawn(cue: Cue) -> Option<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("hermes-cue".into())
        .spawn(move || {
            let _ = blocking(cue);
        })
        .ok()
}

pub(crate) fn play(cue: Cue) {
    let _ = spawn(cue);
}

pub(crate) fn wait(cue: Cue) {
    if let Some(thread) = spawn(cue) {
        let _ = thread.join();
    }
}

fn blocking(cue: Cue) -> Result<()> {
    let device = cpal::default_host()
        .default_output_device()
        .context("no cue output device")?;
    let supported = device.default_output_config()?;
    let config = supported.config();
    let samples = render(cue, config.sample_rate);
    let (done, finished) = mpsc::channel();
    let stream = match supported.sample_format() {
        SampleFormat::F32 => stream::<f32>(&device, &config, samples, done),
        SampleFormat::I16 => stream::<i16>(&device, &config, samples, done),
        SampleFormat::U16 => stream::<u16>(&device, &config, samples, done),
        format => anyhow::bail!("unsupported cue format: {format:?}"),
    }?;
    stream.play()?;
    let _ = finished.recv_timeout(Duration::from_secs(1));
    Ok(())
}

fn stream<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    samples: Vec<f32>,
    done: mpsc::Sender<()>,
) -> Result<cpal::Stream> {
    let channels = usize::from(config.channels);
    let mut next = samples.into_iter();
    Ok(device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            for frame in data.chunks_mut(channels) {
                match next.next() {
                    Some(value) => frame.fill(T::from_sample(value)),
                    None => {
                        frame.fill(T::from_sample(0f32));
                        let _ = done.send(());
                    }
                }
            }
        },
        |_| {},
        None,
    )?)
}
