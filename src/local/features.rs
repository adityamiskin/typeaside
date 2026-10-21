// Adapted from voxtype's cohere_fbank.rs (MIT, Copyright 2025 Peter Jackson).
// See licenses/voxtype-MIT.txt. Framing and Slaney filters follow the HF processor.
use ndarray::Array3;
use rustfft::{FftPlanner, num_complex::Complex};

const FFT_SIZE: usize = 512;

pub(super) struct Features {
    window: Vec<f32>,
    filters: Vec<Vec<f32>>,
}
impl Features {
    pub fn new() -> Self {
        let window = (0..400)
            .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / 399.0).cos())
            .collect();
        let hz_to_mel = |hz: f32| {
            if hz < 1000.0 {
                hz / (200.0 / 3.0)
            } else {
                15.0 + (hz / 1000.0).ln() / (6.4_f32.ln() / 27.0)
            }
        };
        let mel_to_hz = |mel: f32| {
            if mel < 15.0 {
                mel * (200.0 / 3.0)
            } else {
                1000.0 * ((mel - 15.0) * (6.4_f32.ln() / 27.0)).exp()
            }
        };
        let points: Vec<_> = (0..130)
            .map(|i| mel_to_hz(hz_to_mel(8000.0) * i as f32 / 129.0))
            .collect();
        let filters = (0..128)
            .map(|m| {
                (0..=FFT_SIZE / 2)
                    .map(|b| {
                        let hz = b as f32 * 16000.0 / FFT_SIZE as f32;
                        let weight = ((hz - points[m]) / (points[m + 1] - points[m]))
                            .min((points[m + 2] - hz) / (points[m + 2] - points[m + 1]))
                            .max(0.0);
                        weight * 2.0 / (points[m + 2] - points[m])
                    })
                    .collect()
            })
            .collect();
        Self { window, filters }
    }
    pub fn extract(&self, samples: &[f32]) -> Array3<f32> {
        let frames = samples.len() / 160 + 1;
        let mut output = Array3::<f32>::zeros((1, frames, 128));
        let fft = FftPlanner::<f32>::new().plan_fft_forward(FFT_SIZE);
        let mut buffer = vec![Complex::new(0.0, 0.0); FFT_SIZE];
        let sample = |i: isize| {
            if i < 0 {
                0.0
            } else {
                samples.get(i as usize).copied().unwrap_or(0.0)
            }
        };
        for frame in 0..frames {
            for (i, value) in buffer.iter_mut().enumerate() {
                let window_i = i as isize - (FFT_SIZE as isize - 400) / 2;
                let pos = (frame * 160 + i) as isize - FFT_SIZE as isize / 2;
                let mut v = sample(pos);
                if pos > 0 {
                    v -= 0.97 * sample(pos - 1);
                }
                *value = Complex::new(
                    if (0..400).contains(&window_i) {
                        v * self.window[window_i as usize]
                    } else {
                        0.0
                    },
                    0.0,
                );
            }
            fft.process(&mut buffer);
            for (m, filter) in self.filters.iter().enumerate() {
                let energy: f32 = filter
                    .iter()
                    .zip(&buffer)
                    .map(|(w, b)| w * b.norm_sqr())
                    .sum();
                output[[0, frame, m]] = (energy + 2.0_f32.powi(-24)).ln();
            }
        }
        let valid = samples.len() / 160;
        for m in 0..128 {
            let mean = (0..valid).map(|f| output[[0, f, m]]).sum::<f32>() / valid.max(1) as f32;
            let var = (0..valid)
                .map(|f| (output[[0, f, m]] - mean).powi(2))
                .sum::<f32>()
                / valid.saturating_sub(1).max(1) as f32;
            for f in 0..frames {
                output[[0, f, m]] = if f < valid {
                    (output[[0, f, m]] - mean) / (var.sqrt() + 1e-5)
                } else {
                    0.0
                };
            }
        }
        output
    }
}
