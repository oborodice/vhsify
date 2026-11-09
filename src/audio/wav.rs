pub(super) fn read(path: &str) -> (hound::WavSpec, Vec<i16>) {
    let mut reader = hound::WavReader::open(path).expect("Failed to open WAV");
    let spec = reader.spec();
    let samples: Vec<i16> = reader
        .samples::<i16>()
        .map(|s| s.expect("Failed to read sample"))
        .collect();
    (spec, samples)
}

pub(super) fn write(path: &str, spec: hound::WavSpec, samples: Vec<i16>) {
    let mut writer = hound::WavWriter::create(path, spec).expect("Failed to create WAV");
    for sample in samples {
        writer.write_sample(sample).expect("Failed to write sample");
    }
    writer.finalize().expect("Failed to finalize WAV");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ffmpeg が取り出す WAV と同じ、44.1 kHz・16 bit・ステレオ
    const SPEC: hound::WavSpec = hound::WavSpec {
        channels: 2,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    #[test]
    fn writing_then_reading_keeps_the_spec_and_samples() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("audio.wav").to_string_lossy().into_owned();
        // 左右のチャンネルが交互に並ぶ。両端の値も含める
        let samples = vec![0, 1, -1, 1000, i16::MAX, i16::MIN];

        write(&path, SPEC, samples.clone());
        let (spec, read_samples) = read(&path);

        assert_eq!(spec, SPEC);
        assert_eq!(read_samples, samples);
    }
}
