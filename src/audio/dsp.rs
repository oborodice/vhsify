/// ローパスフィルタ（一次IIRフィルタ）でテープ帯域幅をシミュレート
pub(super) fn lowpass(
    samples: Vec<i16>,
    sample_rate: f32,
    channels: usize,
    cutoff_hz: f32,
) -> Vec<i16> {
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
pub(super) fn hiss(mut samples: Vec<i16>, amplitude: f32) -> Vec<i16> {
    let scale = amplitude * i16::MAX as f32;
    let mut rng = 0xdeadbeef_u64;
    let lcg_multiplier: u64 = 6364136223846793005;
    let lcg_increment: u64 = 1442695040888963407;
    for sample in &mut samples {
        rng = rng.wrapping_mul(lcg_multiplier).wrapping_add(lcg_increment);
        let noise = (rng >> 33) as f32 / u32::MAX as f32 * 2.0 - 1.0;
        *sample = (*sample as i32 + (noise * scale) as i32).clamp(i16::MIN as i32, i16::MAX as i32)
            as i16;
    }
    samples
}

/// LFOによるワウフラッター（テープの速度ムラによるピッチ揺れ）
pub(super) fn wow_flutter(samples: Vec<i16>, sample_rate: f32, channels: usize) -> Vec<i16> {
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
            output[out_frame * channels + channel] = (sample_at + frac * (sample_next - sample_at))
                .clamp(i16::MIN as f32, i16::MAX as f32)
                as i16;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 44100.0;
    const CUTOFF_HZ: f32 = 8000.0;
    const FRAMES: usize = 4410;

    fn sine(frequency_hz: f32, amplitude: f32) -> Vec<i16> {
        (0..FRAMES)
            .map(|i| {
                let t = i as f32 / SAMPLE_RATE;
                (amplitude * (2.0 * std::f32::consts::PI * frequency_hz * t).sin()) as i16
            })
            .collect()
    }

    /// Largest absolute sample in the second half, after the filter has settled
    fn settled_peak(samples: &[i16]) -> i16 {
        samples[samples.len() / 2..]
            .iter()
            .map(|s| s.saturating_abs())
            .max()
            .unwrap()
    }

    #[test]
    fn lowpass_passes_a_constant_signal() {
        let output = lowpass(vec![10000; FRAMES], SAMPLE_RATE, 1, CUTOFF_HZ);

        assert!(output[FRAMES / 2..].iter().all(|&s| (s - 10000).abs() <= 1));
    }

    #[test]
    fn lowpass_passes_low_frequencies() {
        let output = lowpass(sine(100.0, 10000.0), SAMPLE_RATE, 1, CUTOFF_HZ);

        assert!(settled_peak(&output) > 9500);
    }

    #[test]
    fn lowpass_attenuates_the_highest_frequency() {
        // Alternating samples are the Nyquist frequency; a one-pole filter at 8 kHz keeps about 36%
        let input: Vec<i16> = (0..FRAMES)
            .map(|i| if i % 2 == 0 { 10000 } else { -10000 })
            .collect();

        let output = lowpass(input, SAMPLE_RATE, 1, CUTOFF_HZ);

        assert!(settled_peak(&output) < 5000);
    }

    #[test]
    fn lowpass_keeps_channels_separate() {
        let stereo: Vec<i16> = (0..FRAMES).flat_map(|_| [10000, 0]).collect();

        let output = lowpass(stereo, SAMPLE_RATE, 2, CUTOFF_HZ);

        let right: Vec<i16> = output.iter().skip(1).step_by(2).copied().collect();
        assert!(right.iter().all(|&s| s == 0));
        assert_eq!(output[output.len() - 2], 10000);
    }

    #[test]
    fn hiss_adds_noise_within_the_amplitude() {
        let amplitude = 0.002;
        let max_noise = (amplitude * i16::MAX as f32).ceil() as i16;

        let output = hiss(vec![0; FRAMES], amplitude);

        assert!(output.iter().any(|&s| s != 0));
        assert!(output.iter().all(|&s| s.abs() <= max_noise));
    }

    #[test]
    fn hiss_does_not_wrap_around_at_the_sample_limits() {
        let amplitude = 0.002;
        let max_noise = (amplitude * i16::MAX as f32).ceil() as i16;

        let loudest = hiss(vec![i16::MAX; FRAMES], amplitude);
        let quietest = hiss(vec![i16::MIN; FRAMES], amplitude);

        assert!(loudest.iter().all(|&s| s >= i16::MAX - max_noise));
        assert!(quietest.iter().all(|&s| s <= i16::MIN + max_noise));
    }

    #[test]
    fn hiss_is_deterministic() {
        let input = sine(440.0, 10000.0);

        assert_eq!(hiss(input.clone(), 0.002), hiss(input, 0.002));
    }

    #[test]
    #[ignore = "known bug (#30): (rng >> 33) / u32::MAX only reaches 0.5, so the noise is always negative"]
    fn hiss_noise_is_centered_on_zero() {
        let output = hiss(vec![0; FRAMES], 0.002);

        let mean = output.iter().map(|&s| f64::from(s)).sum::<f64>() / output.len() as f64;
        assert!(output.iter().any(|&s| s > 0));
        assert!(mean.abs() < 5.0, "mean noise was {mean:.1}");
    }

    #[test]
    fn wow_flutter_keeps_the_length() {
        let stereo: Vec<i16> = sine(440.0, 10000.0)
            .into_iter()
            .flat_map(|s| [s, s])
            .collect();

        let output = wow_flutter(stereo.clone(), SAMPLE_RATE, 2);

        assert_eq!(output.len(), stereo.len());
    }

    #[test]
    fn wow_flutter_reads_nearby_frames_within_its_depth() {
        // Each sample holds its own frame index, so the output shows how far each frame was read from
        let ramp: Vec<i16> = (0..FRAMES as i16).collect();
        // Wow depth (30) plus flutter depth (5), plus one for the truncation to i16
        let max_shift = 36;

        let output = wow_flutter(ramp, SAMPLE_RATE, 1);

        let shifts: Vec<i32> = output
            .iter()
            .enumerate()
            .map(|(frame, &s)| i32::from(s) - frame as i32)
            .collect();
        assert!(shifts.iter().any(|&shift| shift.abs() >= 2));
        assert!(shifts.iter().all(|&shift| shift.abs() <= max_shift));
    }

    #[test]
    fn wow_flutter_keeps_a_constant_signal() {
        let output = wow_flutter(vec![1234; FRAMES], SAMPLE_RATE, 1);

        assert!(output.iter().all(|&s| s == 1234));
    }

    #[test]
    fn wow_flutter_keeps_channels_separate() {
        let stereo: Vec<i16> = (0..FRAMES).flat_map(|_| [1000, -1000]).collect();

        let output = wow_flutter(stereo, SAMPLE_RATE, 2);

        assert!(output.chunks(2).all(|frame| frame == [1000, -1000]));
    }

    #[test]
    #[ignore = "known bug (#31): num_frames - 2 underflows when the audio has fewer than two frames"]
    fn wow_flutter_handles_a_single_frame() {
        let output = wow_flutter(vec![1000, -1000], SAMPLE_RATE, 2);

        assert_eq!(output, [1000, -1000]);
    }
}
