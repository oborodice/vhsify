mod dsp;

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
    let (spec, samples) = read_wav(input_wav);
    let sample_rate = spec.sample_rate as f32;
    let channels = spec.channels as usize;

    let samples = dsp::lowpass(samples, sample_rate, channels, dsp::LOWPASS_CUTOFF_HZ);
    let samples = dsp::hiss(samples, dsp::HISS_AMPLITUDE);
    let samples = dsp::wow_flutter(samples, sample_rate, channels);

    write_wav(output_wav, spec, samples);
}

fn read_wav(path: &str) -> (hound::WavSpec, Vec<i16>) {
    let mut reader = hound::WavReader::open(path).expect("Failed to open WAV");
    let spec = reader.spec();
    let samples: Vec<i16> = reader
        .samples::<i16>()
        .map(|s| s.expect("Failed to read sample"))
        .collect();
    (spec, samples)
}

fn write_wav(path: &str, spec: hound::WavSpec, samples: Vec<i16>) {
    let mut writer = hound::WavWriter::create(path, spec).expect("Failed to create WAV");
    for sample in samples {
        writer.write_sample(sample).expect("Failed to write sample");
    }
    writer.finalize().expect("Failed to finalize WAV");
}
