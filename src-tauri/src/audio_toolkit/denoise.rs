//! Optional RNNoise pass over a finished recording, before transcription.
//! RNNoise operates on 48 kHz, 480-sample frames with ±32768-scale floats, so
//! the 16 kHz pipeline audio is resampled up, denoised, and resampled back.
//! Batch-only by design: the stored recording keeps the original audio, only
//! the transcription input is cleaned.

use super::audio::FrameResampler;
use nnnoiseless::DenoiseState;
use std::time::Duration;

const RNNOISE_RATE: usize = 48_000;
const PIPELINE_RATE: usize = 16_000;
const I16_SCALE: f32 = 32767.0;

fn resample(samples: &[f32], from_hz: usize, to_hz: usize) -> Vec<f32> {
    let mut out = Vec::with_capacity(samples.len() * to_hz / from_hz + 1);
    let mut resampler = FrameResampler::new(from_hz, to_hz, Duration::from_millis(10));
    resampler.push(samples, |frame| out.extend_from_slice(frame));
    resampler.finish(|frame| out.extend_from_slice(frame));
    out
}

/// Suppress steady background noise in 16 kHz mono speech. Output length can
/// differ from the input by a few frames (resampler edges + RNNoise's ~10 ms
/// lookahead) — irrelevant for batch transcription.
pub fn denoise_speech_16k(samples: &[f32]) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }

    let upsampled = resample(samples, PIPELINE_RATE, RNNOISE_RATE);

    let mut state = DenoiseState::new();
    let mut input = [0.0f32; DenoiseState::FRAME_SIZE];
    let mut output = [0.0f32; DenoiseState::FRAME_SIZE];
    let mut denoised = Vec::with_capacity(upsampled.len());

    for chunk in upsampled.chunks(DenoiseState::FRAME_SIZE) {
        for (dst, src) in input.iter_mut().zip(chunk.iter()) {
            *dst = *src * I16_SCALE;
        }
        if chunk.len() < DenoiseState::FRAME_SIZE {
            input[chunk.len()..].fill(0.0);
        }
        state.process_frame(&mut output, &input);
        denoised.extend(output[..chunk.len()].iter().map(|s| s / I16_SCALE));
    }

    resample(&denoised, RNNOISE_RATE, PIPELINE_RATE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_stays_silent() {
        let samples = vec![0.0f32; 16_000];
        let out = denoise_speech_16k(&samples);
        assert!(!out.is_empty());
        let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak < 0.01, "silence gained energy: peak {peak}");
    }

    #[test]
    fn length_roughly_preserved() {
        let samples: Vec<f32> = (0..32_000)
            .map(|i| 0.3 * (i as f32 * 2.0 * std::f32::consts::PI * 220.0 / 16_000.0).sin())
            .collect();
        let out = denoise_speech_16k(&samples);
        let drift = (out.len() as i64 - samples.len() as i64).abs();
        assert!(
            drift < 4_800,
            "length drifted {drift} samples on a 2 s input"
        );
    }

    #[test]
    fn short_and_empty_inputs_do_not_panic() {
        assert!(denoise_speech_16k(&[]).is_empty());
        let _ = denoise_speech_16k(&[0.1; 100]);
    }
}
