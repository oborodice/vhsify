pub(crate) enum Orientation {
    Normal,
    Rotate180,
    Rotate90,
    Rotate270,
}

pub(crate) fn read_orientation(path: &str) -> Orientation {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Orientation::Normal,
    };
    let mut bufreader = std::io::BufReader::new(file);
    let exifreader = exif::Reader::new();
    let Ok(exif) = exifreader.read_from_container(&mut bufreader) else {
        return Orientation::Normal;
    };
    match exif
        .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))
        .unwrap_or(1)
    {
        3 => Orientation::Rotate180,
        6 => Orientation::Rotate90,
        8 => Orientation::Rotate270,
        _ => Orientation::Normal,
    }
}
