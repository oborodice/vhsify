use ntsc_rs::{NtscEffect, settings::standard::UseField, yiq_fielding::Rgb};

use crate::exif;

pub fn process(input_path: &str) -> String {
    let img = image::open(input_path).expect("Failed to open image");
    let img = correct_orientation(img, input_path);
    let img = scale_to_4_3(img);

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

fn scale_to_4_3(img: image::DynamicImage) -> image::DynamicImage {
    let (visible_w, target_h) = (640u32, 480u32);
    let scaled_w = ((img.width() as f32 * target_h as f32) / img.height() as f32).round() as u32;
    let scaled = img.resize_exact(scaled_w, target_h, image::imageops::FilterType::Lanczos3).into_rgb8();
    let canvas_w = scaled_w.max(visible_w);
    let mut canvas = image::RgbImage::new(canvas_w, target_h);
    image::imageops::overlay(&mut canvas, &scaled, ((canvas_w - scaled_w) / 2) as i64, 0);
    let bar_w = canvas_w.saturating_sub(visible_w) / 2;
    let black = image::Rgb([0u8, 0, 0]);
    for py in 0..target_h {
        for px in 0..bar_w {
            canvas.put_pixel(px, py, black);
            canvas.put_pixel(canvas_w - 1 - px, py, black);
        }
    }
    image::DynamicImage::from(canvas)
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
