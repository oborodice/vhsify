mod dsp;
mod wav;

use crate::ffmpeg;

pub(crate) fn extract(input_path: &str, output_wav: &str) -> bool {
    // 音声のない動画では ffmpeg が失敗するので、音声なしとして扱う
    let extracted = ffmpeg::run(&[
        "-i",
        input_path,
        "-vn",
        "-acodec",
        "pcm_s16le",
        "-ar",
        "44100",
        output_wav,
        "-y",
    ]);
    if !extracted {
        return false;
    }

    let wav_header_bytes: u64 = 44;
    std::fs::metadata(output_wav)
        .map(|m| m.len() > wav_header_bytes)
        .unwrap_or(false)
}

pub(crate) fn apply_effects(input_wav: &str, output_wav: &str) -> Result<(), String> {
    let (spec, samples) = wav::read(input_wav)?;
    let sample_rate = spec.sample_rate as f32;
    let channels = spec.channels as usize;

    let samples = dsp::lowpass(samples, sample_rate, channels, dsp::LOWPASS_CUTOFF_HZ);
    let samples = dsp::hiss(samples, dsp::HISS_AMPLITUDE);
    let samples = dsp::wow_flutter(samples, sample_rate, channels);

    wav::write(output_wav, spec, samples)
}
