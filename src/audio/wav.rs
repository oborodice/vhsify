pub(super) fn read(path: &str) -> Result<(hound::WavSpec, Vec<i16>), String> {
    let mut reader = hound::WavReader::open(path).expect("Failed to open WAV");
    let spec = reader.spec();
    let samples: Vec<i16> = reader
        .samples::<i16>()
        .map(|s| s.expect("Failed to read sample"))
        .collect();
    Ok((spec, samples))
}

pub(super) fn write(path: &str, spec: hound::WavSpec, samples: Vec<i16>) -> Result<(), String> {
    let mut writer = hound::WavWriter::create(path, spec).expect("Failed to create WAV");
    for sample in samples {
        writer.write_sample(sample).expect("Failed to write sample");
    }
    writer.finalize().expect("Failed to finalize WAV");
    Ok(())
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

    /// `tmp` の中の `audio.wav` のパス
    fn wav_path(tmp: &tempfile::TempDir) -> String {
        tmp.path().join("audio.wav").to_string_lossy().into_owned()
    }

    #[test]
    fn writing_then_reading_keeps_the_spec_and_samples() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = wav_path(&tmp);
        // 左右のチャンネルが交互に並ぶ。両端の値も含める
        let samples = vec![0, 1, -1, 1000, i16::MAX, i16::MIN];

        write(&path, SPEC, samples.clone()).unwrap();
        let (spec, read_samples) = read(&path).unwrap();

        assert_eq!(spec, SPEC);
        assert_eq!(read_samples, samples);
    }

    #[test]
    fn reading_a_file_that_is_not_a_wav_fails() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = wav_path(&tmp);
        std::fs::write(&path, "not a wav").unwrap();

        assert_eq!(read(&path), Err(format!("Failed to read audio: {path}")));
    }

    #[test]
    fn reading_a_truncated_wav_fails() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = wav_path(&tmp);
        write(&path, SPEC, vec![0; 100]).unwrap();
        // ヘッダーは読めるが、ヘッダーに書かれた長さの途中でサンプルが途切れる
        let bytes = std::fs::read(&path).unwrap();
        std::fs::write(&path, &bytes[..bytes.len() - 51]).unwrap();

        assert_eq!(read(&path), Err(format!("Failed to read audio: {path}")));
    }

    #[test]
    fn writing_into_a_missing_directory_fails() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp
            .path()
            .join("missing")
            .join("audio.wav")
            .to_string_lossy()
            .into_owned();

        assert_eq!(
            write(&path, SPEC, vec![0, 0]),
            Err(format!("Failed to write audio: {path}"))
        );
    }
}
