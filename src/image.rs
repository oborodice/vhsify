use ntsc_rs::{NtscEffect, settings::standard::UseField, yiq_fielding::Rgb};

use crate::exif;

pub fn process(input_path: &str) -> String {
    let img = image::open(input_path).expect("Failed to open image");

    let img = match exif::read_orientation(input_path) {
        3 => img.rotate180(),
        6 => img.rotate90(),
        8 => img.rotate270(),
        _ => img,
    };

    let mut rgb = img.into_rgb8();
    apply_effect(&mut rgb, 0);

    let output_path = crate::make_output_path(input_path);
    rgb.save(&output_path).expect("Failed to save image");
    output_path
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

