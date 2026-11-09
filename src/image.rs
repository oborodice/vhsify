use ntsc_rs::{NtscEffect, settings::standard::UseField, yiq_fielding::Rgb};

use crate::layout::{self, Layout, ScaleMode, TARGET_HEIGHT, TARGET_WIDTH};
use crate::orientation;

pub(crate) fn process(
    input_path: &str,
    scale_mode: ScaleMode,
    output_dir: Option<&str>,
    output_name: Option<&str>,
) -> String {
    let mut img = image::open(input_path).expect("Failed to open image");
    if let Some(orientation) = orientation::read(input_path) {
        img.apply_orientation(orientation);
    }
    let img = scale_to_4_3(img, scale_mode);

    let mut rgb = img.into_rgb8();
    apply_effect(&mut rgb, 0);

    let output_path = crate::utils::make_output_path(input_path, output_dir, output_name);
    rgb.save(&output_path).expect("Failed to save image");
    output_path
}

fn scale_to_4_3(img: image::DynamicImage, scale_mode: ScaleMode) -> image::DynamicImage {
    let scaled_w = layout::scaled_width(img.width(), img.height());
    let scaled = img
        .resize_exact(
            scaled_w,
            TARGET_HEIGHT,
            image::imageops::FilterType::Lanczos3,
        )
        .into_rgb8();
    let laid_out = match Layout::new(scaled_w, scale_mode) {
        Layout::Crop { x } => {
            image::imageops::crop_imm(&scaled, x, 0, TARGET_WIDTH, TARGET_HEIGHT).to_image()
        }
        Layout::Bars { bar_width, .. } => cover_sides(scaled, bar_width),
        Layout::Pad { x } => pad(&scaled, x),
    };
    image::DynamicImage::from(laid_out)
}

pub(crate) fn apply_effect(rgb: &mut image::RgbImage, frame_num: usize) {
    let (width, height) = rgb.dimensions();
    let mut effect = NtscEffect::default();
    effect.use_field = UseField::Both;
    effect.scale.settings.scale_with_video_size = true;
    effect.apply_effect_to_buffer::<Rgb, u8>(
        ntsc_rs::ctx::global(),
        (width as usize, height as usize),
        rgb.as_mut(),
        frame_num,
        [1.0, 1.0],
    );
}

fn cover_sides(mut img: image::RgbImage, bar_width: u32) -> image::RgbImage {
    let (width, height) = img.dimensions();
    let black = image::Rgb([0, 0, 0]);
    for row in 0..height {
        for col in 0..bar_width {
            img.put_pixel(col, row, black);
            img.put_pixel(width - 1 - col, row, black);
        }
    }
    img
}

fn pad(img: &image::RgbImage, x: u32) -> image::RgbImage {
    let mut canvas = image::RgbImage::new(TARGET_WIDTH, TARGET_HEIGHT);
    image::imageops::overlay(&mut canvas, img, x as i64, 0);
    canvas
}
