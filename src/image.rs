use ntsc_rs::{NtscEffect, settings::standard::UseField, yiq_fielding::Rgb};

use crate::exif;

pub fn process(input_path: &str) -> String {
    let img = image::open(input_path).expect("Failed to open image");
    let img = correct_orientation(img, input_path);

    let mut rgb = img.into_rgb8();
    apply_effect(&mut rgb, 0);

    let output_path = crate::utils::make_output_path(input_path);
    rgb.save(&output_path).expect("Failed to save image");
    output_path
}

fn correct_orientation(img: image::DynamicImage, input_path: &str) -> image::DynamicImage {
    match exif::read_orientation(input_path) {
        exif::Orientation::Rotate180 => img.rotate180(),
        exif::Orientation::Rotate90 => img.rotate90(),
        exif::Orientation::Rotate270 => img.rotate270(),
        exif::Orientation::Normal => img,
    }
}

pub(crate) fn apply_effect(rgb: &mut image::RgbImage, frame_num: usize) {
    let (width, height) = rgb.dimensions();
    let mut effect = NtscEffect::default();
    effect.use_field = UseField::Both;
    if let Some(scale) = effect.scale.as_mut() {
        scale.scale_with_video_size = true;
    }
    effect.apply_effect_to_buffer::<Rgb, u8>(
        (width as usize, height as usize),
        rgb.as_mut(),
        frame_num,
        [1.0, 1.0],
    );
}
