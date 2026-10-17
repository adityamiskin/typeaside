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

/// Mono samples for `cue` at `rate` Hz: deep, muffled thucks with no click or bright overtones.
pub fn render(cue: Cue, rate: u32) -> Vec<f32> {
    let thunks: &[Thunk] = match cue {
        Cue::Start => &[Thunk {
            at: 0.,
            pitch: 60.,
            gain: 1.,
        }],
        Cue::Stop => &[Thunk {
            at: 0.,
            pitch: 48.,
            gain: 0.85,
        }],
        Cue::Error => &[
            Thunk {
                at: 0.,
                pitch: 55.,
                gain: 0.85,
            },
            Thunk {
                at: 0.14,
                pitch: 44.,
                gain: 0.8,
            },
        ],
    };
    let rate = rate as f32;
    let length = thunks.iter().map(|thunk| thunk.at).fold(0., f32::max) + 0.4;
    let mut samples = vec![0f32; (length * rate) as usize];
    for thunk in thunks {
        add_thunk(&mut samples, rate, thunk);
    }
    // Two one-pole low-passes at 250 Hz keep it a felt thud on headphones.
    let k = 1. - (-TAU * 250. / rate).exp();
    let (mut low, mut lower) = (0f32, 0f32);
    for sample in &mut samples {
        low += (*sample - low) * k;
        lower += (low - lower) * k;
        *sample = (lower * 1.3).tanh();
    }
    let peak = samples.iter().fold(0f32, |peak, s| peak.max(s.abs()));
    let fade = 0.06 * rate;
    let total = samples.len();
    for (index, sample) in samples.iter_mut().enumerate() {
        *sample *= 0.55 / peak * ((total - index) as f32 / fade).min(1.);
    }
    samples
}

fn add_thunk(samples: &mut [f32], rate: f32, thunk: &Thunk) {
    let start = (thunk.at * rate) as usize;
    let mut phase = 0f32;
    for (n, sample) in samples.iter_mut().skip(start).enumerate() {
        let t = n as f32 / rate;
        // The pitch falls into place like a struck drum, which reads as "thuck" not "beep".
        phase += TAU * thunk.pitch * (1. + 1.5 * (-t / 0.02).exp()) / rate;
        // A 5 ms attack avoids a click; the octave keeps it audible on small speakers.
        let attack = (t / 0.005).min(1.);
        let body = (phase.sin() + 0.25 * (2. * phase).sin()) * (-t / 0.09).exp();
        *sample += body * attack * thunk.gain;
    }
}

fn spawn(cue: Cue) -> Option<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("typeaside-cue".into())
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
