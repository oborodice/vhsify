pub(crate) fn read_orientation(path: &str) -> u32 {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return 1,
    };
    let mut bufreader = std::io::BufReader::new(file);
    let exifreader = exif::Reader::new();
    let Ok(exif) = exifreader.read_from_container(&mut bufreader) else {
        return 1;
    };
    exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
        .unwrap_or(1)
}
