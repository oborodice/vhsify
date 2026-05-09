use std::process::Command;

const LOWPASS_CUTOFF_HZ: f32 = 8000.0;
const HISS_AMPLITUDE: f32 = 0.002;
const WOW_RATE_HZ: f32 = 0.7;
const WOW_DEPTH_SAMPLES: f32 = 30.0;
const FLUTTER_RATE_HZ: f32 = 7.0;
const FLUTTER_DEPTH_SAMPLES: f32 = 5.0;

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

    // WAVヘッダは44バイト。それ以上あれば音声データあり
    std::fs::metadata(output_wav)
        .map(|m| m.len() > 44)
        .unwrap_or(false)
}

pub(crate) fn apply_effects(input_wav: &str, output_wav: &str) {
    let (spec, samples) = read_wav(input_wav);
    let sample_rate = spec.sample_rate as f32;
    let channels = spec.channels as usize;

    let samples = lowpass(samples, sample_rate, channels, LOWPASS_CUTOFF_HZ);
    let samples = hiss(samples, HISS_AMPLITUDE);
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
    for s in samples {
        writer.write_sample(s).expect("Failed to write sample");
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

    for (i, &s) in samples.iter().enumerate() {
        let ch = i % channels;
        let y = alpha * s as f32 + (1.0 - alpha) * prev[ch];
        prev[ch] = y;
        output[i] = y.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
    }
    output
}

/// ホワイトノイズによるヒスノイズ付加
fn hiss(mut samples: Vec<i16>, amplitude: f32) -> Vec<i16> {
    let mut rng = 0xdeadbeef_u64;
    let scale = amplitude * i16::MAX as f32;
    for s in &mut samples {
        rng = rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let noise = (rng >> 33) as f32 / u32::MAX as f32 * 2.0 - 1.0;
        *s = (*s as i32 + (noise * scale) as i32)
            .clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    }
    samples
}

/// LFOによるワウフラッター（テープの速度ムラによるピッチ揺れ）
fn wow_flutter(samples: Vec<i16>, sample_rate: f32, channels: usize) -> Vec<i16> {
    let num_frames = samples.len() / channels;
    let mut output = vec![0i16; samples.len()];
    let pi2 = 2.0 * std::f32::consts::PI;

    for out_frame in 0..num_frames {
        let t = out_frame as f32 / sample_rate;
        let offset = WOW_DEPTH_SAMPLES * (pi2 * WOW_RATE_HZ * t).sin()
            + FLUTTER_DEPTH_SAMPLES * (pi2 * FLUTTER_RATE_HZ * t).sin();
        let src = (out_frame as f32 + offset).clamp(0.0, (num_frames - 2) as f32);
        let i0 = src as usize;
        let frac = src - i0 as f32;

        for ch in 0..channels {
            let s0 = samples[i0 * channels + ch] as f32;
            let s1 = samples[(i0 + 1) * channels + ch] as f32;
            output[out_frame * channels + ch] =
                (s0 + frac * (s1 - s0)).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        }
    }
    output
}
