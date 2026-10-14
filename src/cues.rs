use anyhow::{Context, Result};
use cpal::{
    FromSample, SampleFormat, SizedSample,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use std::time::Duration;

#[derive(Clone, Copy)]
pub(crate) enum Cue {
    Start,
    Stop,
    Error,
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

pub(crate) fn blocking(cue: Cue) -> Result<()> {
    let device = cpal::default_host()
        .default_output_device()
        .context("no cue output device")?;
    let supported = device.default_output_config()?;
    let config = supported.config();
    let stream = match supported.sample_format() {
        SampleFormat::F32 => stream::<f32>(&device, &config, cue),
        SampleFormat::I16 => stream::<i16>(&device, &config, cue),
        SampleFormat::U16 => stream::<u16>(&device, &config, cue),
        format => anyhow::bail!("unsupported cue format: {format:?}"),
    }?;
    stream.play()?;
    std::thread::sleep(Duration::from_millis(if matches!(cue, Cue::Error) {
        75
    } else {
        120
    }));
    drop(stream);
    Ok(())
}

fn stream<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    cue: Cue,
) -> Result<cpal::Stream> {
    let rate = config.sample_rate as f32;
    let channels = usize::from(config.channels);
    let mut frame = 0usize;
    let frequencies: &[f32] = match cue {
        Cue::Start => &[660., 880.],
        Cue::Stop => &[880., 660.],
        Cue::Error => &[330.],
    };
    Ok(device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            for samples in data.chunks_mut(channels) {
                let t = frame as f32 / rate;
                let blip = (t / 0.055) as usize;
                let local = t - blip as f32 * 0.055;
                let value = if local < 0.045 {
                    frequencies
                        .get(blip)
                        .map(|frequency| {
                            let fade = (local / 0.005).min((0.045 - local) / 0.005).clamp(0., 1.);
                            (std::f32::consts::TAU * frequency * local).sin() * 0.12 * fade
                        })
                        .unwrap_or(0.)
                } else {
                    0.
                };
                samples.fill(T::from_sample(value));
                frame += 1;
            }
        },
        |_| {},
        None,
    )?)
}
