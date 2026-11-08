mod common;

use std::ops::Range;
use std::path::{Path, PathBuf};

use image::{GrayImage, Rgb, RgbImage};
use tempfile::TempDir;

// The VHS effect adds noise, so compare region averages instead of exact pixels.
// Measured averages are about 1 for black bars and about 250 for white content.
const BLACK_MAX: f64 = 20.0;
const WHITE_MIN: f64 = 200.0;
// Measured std dev of luma on a flat gray image is about 8 with the effect and 0 without it.
const NOISE_STD_DEV_MIN: f64 = 2.0;

// Columns holding the image content in 640x480 output: full width for landscape, between bars for portrait
const LANDSCAPE_COLUMNS: Range<u32> = 0..640;
const PORTRAIT_COLUMNS: Range<u32> = 140..500;

#[test]
fn wide_image_keeps_scaled_width_and_covers_sides_with_bars_by_default() {
    for mode_args in [&[][..], &["--mode", "bars"][..]] {
        println!("mode args: {mode_args:?}");
        let output = convert_white(1920, 1080, mode_args);

        // 1920x1080 scales to 853x480; bars cover 106px on each side to leave a 4:3 area
        assert_eq!(output.dimensions(), (853, 480));
        assert_black(&output, 0..100);
        assert_white(&output, 112..741);
        assert_black(&output, 753..853);
    }
}

#[test]
fn wide_image_is_cropped_to_4_3_in_crop_mode() {
    for mode_args in [&["--mode", "crop"][..], &["-m", "crop"][..]] {
        println!("mode args: {mode_args:?}");
        let output = convert_white(1920, 1080, mode_args);

        assert_eq!(output.dimensions(), (640, 480));
        assert_white(&output, 4..636);
    }
}

#[test]
fn four_by_three_image_has_no_bars_in_any_mode() {
    for mode in ["bars", "crop"] {
        println!("mode: {mode}");
        let output = convert_white(1280, 960, &["--mode", mode]);

        assert_eq!(output.dimensions(), (640, 480));
        assert_white(&output, 4..636);
    }
}

#[test]
fn tall_image_is_padded_with_bars_in_any_mode() {
    for mode in ["bars", "crop"] {
        println!("mode: {mode}");
        let output = convert_white(480, 640, &["--mode", mode]);

        // 480x640 scales to 360x480 and is centered with 140px bars
        assert_eq!(output.dimensions(), (640, 480));
        assert_portrait_bars(&output);
        assert_white(&output, 146..494);
    }
}

#[test]
fn square_image_is_padded_with_bars() {
    let output = convert_white(500, 500, &[]);

    // 500x500 scales to 480x480 and is centered with 80px bars
    assert_eq!(output.dimensions(), (640, 480));
    assert_black(&output, 0..74);
    assert_white(&output, 86..554);
    assert_black(&output, 566..640);
}

// EXIF tests convert a 640x480 JPEG whose top-left quadrant is white and check where that quadrant ends up.
// Rotated images become 480x640, which scales to 360x480 between 140px bars.

#[test]
fn exif_orientation_normal_keeps_image_as_is() {
    let output = convert_with_orientation(1);

    assert_eq!(output.dimensions(), (640, 480));
    assert_white_corner(&output, LANDSCAPE_COLUMNS, Corner::TopLeft);
}

#[test]
fn exif_orientation_invalid_value_keeps_image_as_is() {
    let output = convert_with_orientation(9);

    assert_eq!(output.dimensions(), (640, 480));
    assert_white_corner(&output, LANDSCAPE_COLUMNS, Corner::TopLeft);
}

#[test]
fn exif_orientation_flip_horizontal_moves_top_left_to_top_right() {
    let output = convert_with_orientation(2);

    assert_eq!(output.dimensions(), (640, 480));
    assert_white_corner(&output, LANDSCAPE_COLUMNS, Corner::TopRight);
}

#[test]
fn exif_orientation_rotate_180_turns_image_upside_down() {
    let output = convert_with_orientation(3);

    assert_eq!(output.dimensions(), (640, 480));
    assert_white_corner(&output, LANDSCAPE_COLUMNS, Corner::BottomRight);
}

#[test]
fn exif_orientation_flip_vertical_moves_top_left_to_bottom_left() {
    let output = convert_with_orientation(4);

    assert_eq!(output.dimensions(), (640, 480));
    assert_white_corner(&output, LANDSCAPE_COLUMNS, Corner::BottomLeft);
}

#[test]
fn exif_orientation_rotate_90_flip_horizontal_keeps_top_left_in_portrait() {
    let output = convert_with_orientation(5);

    assert_eq!(output.dimensions(), (640, 480));
    assert_portrait_bars(&output);
    assert_white_corner(&output, PORTRAIT_COLUMNS, Corner::TopLeft);
}

#[test]
fn exif_orientation_rotate_90_moves_top_left_to_top_right() {
    let output = convert_with_orientation(6);

    assert_eq!(output.dimensions(), (640, 480));
    assert_portrait_bars(&output);
    assert_white_corner(&output, PORTRAIT_COLUMNS, Corner::TopRight);
}

#[test]
fn exif_orientation_rotate_270_flip_horizontal_moves_top_left_to_bottom_right() {
    let output = convert_with_orientation(7);

    assert_eq!(output.dimensions(), (640, 480));
    assert_portrait_bars(&output);
    assert_white_corner(&output, PORTRAIT_COLUMNS, Corner::BottomRight);
}

#[test]
fn exif_orientation_rotate_270_moves_top_left_to_bottom_left() {
    let output = convert_with_orientation(8);

    assert_eq!(output.dimensions(), (640, 480));
    assert_portrait_bars(&output);
    assert_white_corner(&output, PORTRAIT_COLUMNS, Corner::BottomLeft);
}

#[test]
fn jpg_input_is_saved_as_jpg() {
    assert_format_is_kept("jpg");
}

#[test]
fn jpeg_input_is_saved_as_jpeg() {
    assert_format_is_kept("jpeg");
}

#[test]
fn png_input_is_saved_as_png() {
    assert_format_is_kept("png");
}

#[test]
fn webp_input_is_saved_as_webp() {
    assert_format_is_kept("webp");
}

#[test]
fn avif_input_is_saved_as_avif() {
    assert_format_is_kept("avif");
}

#[test]
fn vhs_effect_adds_noise_to_flat_image() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.png");
    // Mid gray so that noise is not clipped at 0 or 255
    RgbImage::from_pixel(640, 480, Rgb([128, 128, 128]))
        .save(&input)
        .unwrap();

    let output = convert(tmp.path(), &input, &[]);

    let std_dev = std_dev_luma(&output);
    assert!(
        std_dev > NOISE_STD_DEV_MIN,
        "expected noise from the VHS effect, but std dev of luma was {std_dev:.2}"
    );
}

fn assert_format_is_kept(ext: &str) {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join(format!("input.{ext}"));
    common::write_image(&input, 64, 48);

    let output_path = run(tmp.path(), &input, &[]);

    assert_eq!(output_path, tmp.path().join(format!("input_vhs.{ext}")));
    assert_eq!(image::image_dimensions(&output_path).unwrap(), (640, 480));
}

/// Converts a 640x480 JPEG whose top-left quadrant is white and the rest is black, tagged with the given EXIF orientation
#[track_caller]
fn convert_with_orientation(orientation: u16) -> GrayImage {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.jpg");
    let img = RgbImage::from_fn(640, 480, |x, y| {
        if x < 320 && y < 240 {
            Rgb([255, 255, 255])
        } else {
            Rgb([0, 0, 0])
        }
    });
    write_jpeg_with_orientation(&input, &img, orientation);

    convert(tmp.path(), &input, &[])
}

fn write_jpeg_with_orientation(path: &Path, img: &RgbImage, orientation: u16) {
    let mut jpeg = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut jpeg),
        image::ImageFormat::Jpeg,
    )
    .unwrap();

    // Minimal EXIF APP1 segment: big-endian TIFF header and a single IFD entry for Orientation
    let mut app1 = vec![0xFF, 0xE1, 0x00, 0x22];
    app1.extend_from_slice(b"Exif\0\0");
    app1.extend_from_slice(&[b'M', b'M', 0x00, 0x2A, 0x00, 0x00, 0x00, 0x08]);
    app1.extend_from_slice(&[0x00, 0x01]);
    app1.extend_from_slice(&[0x01, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01]);
    app1.extend_from_slice(&orientation.to_be_bytes());
    app1.extend_from_slice(&[0x00, 0x00]);
    app1.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

    // Insert the segment right after the SOI marker
    let mut bytes = jpeg[..2].to_vec();
    bytes.extend(app1);
    bytes.extend(&jpeg[2..]);
    std::fs::write(path, bytes).unwrap();
}

#[track_caller]
fn convert_white(width: u32, height: u32, args: &[&str]) -> GrayImage {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.png");
    common::write_image(&input, width, height);

    convert(tmp.path(), &input, args)
}

#[track_caller]
fn convert(cwd: &Path, input: &Path, args: &[&str]) -> GrayImage {
    let output_path = run(cwd, input, args);
    image::open(output_path).unwrap().into_luma8()
}

#[track_caller]
fn run(cwd: &Path, input: &Path, args: &[&str]) -> PathBuf {
    let output = common::vhsify(cwd).arg(input).args(args).output().unwrap();
    assert!(output.status.success(), "{}", common::stderr(&output));

    let stdout = common::stdout(&output);
    let path = stdout
        .trim()
        .strip_prefix("Saved: ")
        .unwrap_or_else(|| panic!("unexpected stdout: {stdout}"));
    PathBuf::from(path)
}

#[derive(PartialEq)]
enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Checks that only the given quadrant of the content columns is white and the other three are black
#[track_caller]
fn assert_white_corner(img: &GrayImage, content: Range<u32>, white: Corner) {
    // Stay away from the quadrant edges, where the VHS effect blurs white into black
    let margin = 6;
    let mid_x = (content.start + content.end) / 2;
    let mid_y = img.height() / 2;
    let left = content.start + margin..mid_x - margin;
    let right = mid_x + margin..content.end - margin;
    let top = margin..mid_y - margin;
    let bottom = mid_y + margin..img.height() - margin;

    for (corner, columns, rows) in [
        (Corner::TopLeft, left.clone(), top.clone()),
        (Corner::TopRight, right.clone(), top),
        (Corner::BottomLeft, left, bottom.clone()),
        (Corner::BottomRight, right, bottom),
    ] {
        if corner == white {
            assert_white_area(img, columns, rows);
        } else {
            assert_black_area(img, columns, rows);
        }
    }
}

#[track_caller]
fn assert_portrait_bars(img: &GrayImage) {
    assert_black(img, 0..134);
    assert_black(img, 506..640);
}

#[track_caller]
fn assert_black(img: &GrayImage, columns: Range<u32>) {
    assert_black_area(img, columns, 0..img.height());
}

#[track_caller]
fn assert_white(img: &GrayImage, columns: Range<u32>) {
    assert_white_area(img, columns, 0..img.height());
}

#[track_caller]
fn assert_black_area(img: &GrayImage, columns: Range<u32>, rows: Range<u32>) {
    let mean = mean_luma(img, columns.clone(), rows.clone());
    assert!(
        mean < BLACK_MAX,
        "expected black at columns {columns:?}, rows {rows:?}, but mean luma was {mean:.1}"
    );
}

#[track_caller]
fn assert_white_area(img: &GrayImage, columns: Range<u32>, rows: Range<u32>) {
    let mean = mean_luma(img, columns.clone(), rows.clone());
    assert!(
        mean > WHITE_MIN,
        "expected white at columns {columns:?}, rows {rows:?}, but mean luma was {mean:.1}"
    );
}

fn mean_luma(img: &GrayImage, columns: Range<u32>, rows: Range<u32>) -> f64 {
    let count = columns.len() * rows.len();
    let sum: f64 = rows
        .flat_map(|y| columns.clone().map(move |x| (x, y)))
        .map(|(x, y)| img.get_pixel(x, y)[0] as f64)
        .sum();
    sum / count as f64
}

fn std_dev_luma(img: &GrayImage) -> f64 {
    let mean = mean_luma(img, 0..img.width(), 0..img.height());
    let variance = img
        .pixels()
        .map(|p| (p[0] as f64 - mean).powi(2))
        .sum::<f64>()
        / (img.width() * img.height()) as f64;
    variance.sqrt()
}
