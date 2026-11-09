mod dsp;
mod wav;

use std::process::Command;

pub(crate) fn extract(input_path: &str, output_wav: &str) -> bool {
    let status = Command::new("ffmpeg")
        .args([
            "-i",
            input_path,
            "-vn",
            "-acodec",
            "pcm_s16le",
            "-ar",
            "44100",
            output_wav,
            "-y",
        ])
        .status()
        .expect("Failed to run ffmpeg");

    if !status.success() {
        return false;
    }

    let wav_header_bytes: u64 = 44;
    std::fs::metadata(output_wav)
        .map(|m| m.len() > wav_header_bytes)
        .unwrap_or(false)
}

pub(crate) fn apply_effects(input_wav: &str, output_wav: &str) {
    let (spec, samples) = wav::read(input_wav);
    let sample_rate = spec.sample_rate as f32;
    let channels = spec.channels as usize;

    let samples = dsp::lowpass(samples, sample_rate, channels, dsp::LOWPASS_CUTOFF_HZ);
    let samples = dsp::hiss(samples, dsp::HISS_AMPLITUDE);
    let samples = dsp::wow_flutter(samples, sample_rate, channels);

    wav::write(output_wav, spec, samples);
}
