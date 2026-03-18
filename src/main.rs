use std::env;

use ntsc_rs::{NtscEffect, settings::standard::UseField, yiq_fielding::Rgb};

fn read_jpeg_orientation(path: &str) -> exif::Value {
    let file = std::fs::File::open(path).unwrap();
    let mut bufreader = std::io::BufReader::new(file);
    let exifreader = exif::Reader::new();
    let Ok(exif) = exifreader.read_from_container(&mut bufreader) else {
        return exif::Value::Short(vec![1]);
    };
    exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .map(|f| f.value.clone())
        .unwrap_or(exif::Value::Short(vec![1]))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: vhsify <INPUT>");
        std::process::exit(1);
    }

    let input_path = &args[1];
    let img = image::open(input_path).expect("Failed to open image");

    let orientation = read_jpeg_orientation(input_path);
    let img = match orientation.get_uint(0) {
        Some(3) => img.rotate180(),
        Some(6) => img.rotate90(),
        Some(8) => img.rotate270(),
        _ => img,
    };

    let mut rgb = img.into_rgb8();
    let (width, height) = rgb.dimensions();

    let mut effect = NtscEffect::default();
    effect.use_field = UseField::Both;
    if let Some(scale) = effect.scale.as_mut() {
        scale.scale_with_video_size = true;
    }
    effect.apply_effect_to_buffer::<Rgb, u8>(
        (width as usize, height as usize),
        rgb.as_mut(),
        0,
        [1.0, 1.0],
    );

    let stem = std::path::Path::new(input_path)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap();
    let ext = std::path::Path::new(input_path)
        .extension()
        .unwrap()
        .to_str()
        .unwrap();
    let output_path = format!("{}_vhs.{}", stem, ext);

    rgb.save(&output_path).expect("Failed to save image");
    println!("Saved: {}", output_path);
}
