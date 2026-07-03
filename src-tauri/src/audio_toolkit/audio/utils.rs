use anyhow::Result;
use hound::{WavReader, WavSpec, WavWriter};
use log::debug;
use std::path::Path;

/// Read a WAV file and return normalised f32 samples.
pub fn read_wav_samples<P: AsRef<Path>>(file_path: P) -> Result<Vec<f32>> {
    let reader = WavReader::open(file_path.as_ref())?;
    let samples = reader
        .into_samples::<i16>()
        .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
        .collect::<Result<Vec<f32>, _>>()?;
    Ok(samples)
}

/// Verify a WAV file by reading it back and checking the sample count.
pub fn verify_wav_file<P: AsRef<Path>>(file_path: P, expected_samples: usize) -> Result<()> {
    let reader = WavReader::open(file_path.as_ref())?;
    let actual_samples = reader.len() as usize;
    if actual_samples != expected_samples {
        anyhow::bail!(
            "WAV sample count mismatch: expected {}, got {}",
            expected_samples,
            actual_samples
        );
    }
    Ok(())
}

/// Save audio samples as a WAV file
pub fn save_wav_file<P: AsRef<Path>>(file_path: P, samples: &[f32]) -> Result<()> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(file_path.as_ref(), spec)?;

    // Convert f32 samples to i16 for WAV
    for sample in samples {
        let sample_i16 = (sample * i16::MAX as f32) as i16;
        writer.write_sample(sample_i16)?;
    }

    writer.finalize()?;
    debug!("Saved WAV file: {:?}", file_path.as_ref());
    Ok(())
}

/// Estimate the speech level of a recording as the 90th-percentile RMS of
/// 100 ms frames, in dBFS. A percentile (not the mean) so leading/trailing
/// silence cannot mask a too-quiet voice; `None` when the recording is
/// shorter than one frame.
pub fn speech_level_dbfs(samples: &[f32], sample_rate: u32) -> Option<f32> {
    let frame_len = (sample_rate as usize / 10).max(1);
    let mut frame_rms: Vec<f32> = samples
        .chunks(frame_len)
        .filter(|chunk| chunk.len() == frame_len)
        .map(|chunk| (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt())
        .collect();
    if frame_rms.is_empty() {
        return None;
    }
    frame_rms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let index = ((frame_rms.len() - 1) as f32 * 0.9).round() as usize;
    let level = frame_rms[index].max(1e-9);
    Some(20.0 * level.log10())
}

#[cfg(test)]
mod speech_level_tests {
    use super::*;

    #[test]
    fn silence_reports_far_below_any_threshold() {
        let samples = vec![0.0f32; 16_000];
        let level = speech_level_dbfs(&samples, 16_000).unwrap();
        assert!(level < -100.0, "silence measured at {level} dBFS");
    }

    #[test]
    fn full_scale_sine_is_near_minus_three_dbfs() {
        let samples: Vec<f32> = (0..16_000)
            .map(|i| (i as f32 * 2.0 * std::f32::consts::PI * 440.0 / 16_000.0).sin())
            .collect();
        let level = speech_level_dbfs(&samples, 16_000).unwrap();
        assert!(
            (level + 3.0).abs() < 0.5,
            "sine RMS measured at {level} dBFS"
        );
    }

    #[test]
    fn quiet_speech_with_silence_padding_is_not_masked() {
        // 3 s of silence around 1 s of a quiet (-40 dBFS RMS) tone: the
        // percentile must report the tone, not the silence-dominated mean.
        let amplitude = 0.014_f32; // RMS ~0.01 → ~-40 dBFS
        let mut samples = vec![0.0f32; 24_000];
        samples.extend(
            (0..16_000).map(|i| {
                amplitude * (i as f32 * 2.0 * std::f32::consts::PI * 200.0 / 16_000.0).sin()
            }),
        );
        samples.extend(vec![0.0f32; 24_000]);
        let level = speech_level_dbfs(&samples, 16_000).unwrap();
        assert!((-42.0..=-38.0).contains(&level), "measured {level} dBFS");
    }

    #[test]
    fn too_short_recording_returns_none() {
        assert!(speech_level_dbfs(&[0.5; 100], 16_000).is_none());
    }
}
