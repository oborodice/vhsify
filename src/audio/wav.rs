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
