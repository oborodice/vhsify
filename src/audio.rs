use std::process::Command;

pub(crate) fn extract(input_path: &str, output_wav: &str) -> bool {
    let status = Command::new("ffmpeg")
        .args([
            "-i", input_path,
            "-vn",
            "-acodec", "pcm_s16le",
            "-ar", "44100",
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

    let lowpass_cutoff_hz = 8000.0;
    let hiss_amplitude = 0.002;
    let samples = lowpass(samples, sample_rate, channels, lowpass_cutoff_hz);
    let samples = hiss(samples, hiss_amplitude);
    let samples = wow_flutter(samples, sample_rate, channels);

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

/// ローパスフィルタ（一次IIRフィルタ）でテープ帯域幅をシミュレート
fn lowpass(samples: Vec<i16>, sample_rate: f32, channels: usize, cutoff_hz: f32) -> Vec<i16> {
    let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff_hz);
    let dt = 1.0 / sample_rate;
    let alpha = dt / (rc + dt);

    let mut output = vec![0i16; samples.len()];
    let mut prev = vec![0.0f32; channels];

    for (i, &sample) in samples.iter().enumerate() {
        let channel = i % channels;
        let filtered = alpha * sample as f32 + (1.0 - alpha) * prev[channel];
        prev[channel] = filtered;
        output[i] = filtered.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
    }
    output
}

/// ホワイトノイズによるヒスノイズ付加
fn hiss(mut samples: Vec<i16>, amplitude: f32) -> Vec<i16> {
    let scale = amplitude * i16::MAX as f32;
    let mut rng = 0xdeadbeef_u64;
    let lcg_multiplier: u64 = 6364136223846793005;
    let lcg_increment: u64 = 1442695040888963407;
    for sample in &mut samples {
        rng = rng
            .wrapping_mul(lcg_multiplier)
            .wrapping_add(lcg_increment);
        let noise = (rng >> 33) as f32 / u32::MAX as f32 * 2.0 - 1.0;
        *sample = (*sample as i32 + (noise * scale) as i32)
            .clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    }
    samples
}

/// LFOによるワウフラッター（テープの速度ムラによるピッチ揺れ）
fn wow_flutter(samples: Vec<i16>, sample_rate: f32, channels: usize) -> Vec<i16> {
    let wow_rate_hz = 0.7;
    let wow_depth_samples = 30.0;
    let flutter_rate_hz = 7.0;
    let flutter_depth_samples = 5.0;

    let num_frames = samples.len() / channels;
    let mut output = vec![0i16; samples.len()];
    let pi2 = 2.0 * std::f32::consts::PI;

    for out_frame in 0..num_frames {
        let t = out_frame as f32 / sample_rate;
        let offset = wow_depth_samples * (pi2 * wow_rate_hz * t).sin()
            + flutter_depth_samples * (pi2 * flutter_rate_hz * t).sin();
        let src = (out_frame as f32 + offset).clamp(0.0, (num_frames - 2) as f32);
        let src_index = src as usize;
        let frac = src - src_index as f32;

        for channel in 0..channels {
            let sample_at = samples[src_index * channels + channel] as f32;
            let sample_next = samples[(src_index + 1) * channels + channel] as f32;
            output[out_frame * channels + channel] =
                (sample_at + frac * (sample_next - sample_at)).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        }
    }
    output
}
