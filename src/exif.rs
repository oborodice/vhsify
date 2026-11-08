use image::metadata::Orientation;

pub(crate) fn read_orientation(path: &str) -> Option<Orientation> {
    let file = std::fs::File::open(path).ok()?;
    let exif = exif::Reader::new()
        .read_from_container(&mut std::io::BufReader::new(file))
        .ok()?;
    let value = exif
        .get_field(exif::Tag::Orientation, exif::In::PRIMARY)?
        .value
        .get_uint(0)?;
    Orientation::from_exif(u8::try_from(value).ok()?)
}
