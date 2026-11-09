mod common;

use std::ops::Range;
use std::path::Path;

use image::{GrayImage, Rgb, RgbImage};
use tempfile::TempDir;

use common::luma::{
    NOISE_STD_DEV_MIN, assert_black, assert_black_area, assert_portrait_bars, assert_white,
    assert_white_area, std_dev_luma,
};

// 640x480 の出力で画像の内容がある列。横長なら全幅、縦長なら黒帯の間
const LANDSCAPE_COLUMNS: Range<u32> = 0..640;
const PORTRAIT_COLUMNS: Range<u32> = 140..500;

#[test]
fn wide_image_keeps_scaled_width_and_covers_sides_with_bars_by_default() {
    for mode_args in [&[][..], &["--mode", "bars"][..]] {
        println!("mode args: {mode_args:?}");
        let output = convert_white(1920, 1080, mode_args);

        // 1920x1080 は 854x480 に縮小され、左右 107px ずつを黒帯で覆って 4:3 の範囲を残す
        assert_eq!(output.dimensions(), (854, 480));
        assert_black(&output, 0..100);
        assert_white(&output, 112..741);
        assert_black(&output, 753..854);
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

        // 480x640 は 360x480 に縮小され、140px の黒帯で中央に置かれる
        assert_eq!(output.dimensions(), (640, 480));
        assert_portrait_bars(&output);
        assert_white(&output, 146..494);
    }
}

#[test]
fn square_image_is_padded_with_bars() {
    let output = convert_white(500, 500, &[]);

    // 500x500 は 480x480 に縮小され、80px の黒帯で中央に置かれる
    assert_eq!(output.dimensions(), (640, 480));
    assert_black(&output, 0..74);
    assert_white(&output, 86..554);
    assert_black(&output, 566..640);
}

// EXIF のテストでは、左上の 4 分の 1 だけが白い 640x480 の JPEG を変換し、その白い部分がどこに来るかを確かめる。
// 回転した画像は 480x640 になり、360x480 に縮小されて 140px の黒帯の間に置かれる

#[test]
fn exif_orientation_normal_keeps_image_as_is() {
    let output = convert_jpeg_with_orientation("input.jpg", 1);

    assert_eq!(output.dimensions(), (640, 480));
    assert_white_corner(&output, LANDSCAPE_COLUMNS, Corner::TopLeft);
}

#[test]
fn exif_orientation_invalid_value_keeps_image_as_is() {
    let output = convert_jpeg_with_orientation("input.jpg", 9);

    assert_eq!(output.dimensions(), (640, 480));
    assert_white_corner(&output, LANDSCAPE_COLUMNS, Corner::TopLeft);
}

#[test]
fn exif_orientation_flip_horizontal_moves_top_left_to_top_right() {
    let output = convert_jpeg_with_orientation("input.jpg", 2);

    assert_eq!(output.dimensions(), (640, 480));
    assert_white_corner(&output, LANDSCAPE_COLUMNS, Corner::TopRight);
}

#[test]
fn exif_orientation_rotate_180_turns_image_upside_down() {
    let output = convert_jpeg_with_orientation("input.jpg", 3);

    assert_eq!(output.dimensions(), (640, 480));
    assert_white_corner(&output, LANDSCAPE_COLUMNS, Corner::BottomRight);
}

#[test]
fn exif_orientation_flip_vertical_moves_top_left_to_bottom_left() {
    let output = convert_jpeg_with_orientation("input.jpg", 4);

    assert_eq!(output.dimensions(), (640, 480));
    assert_white_corner(&output, LANDSCAPE_COLUMNS, Corner::BottomLeft);
}

#[test]
fn exif_orientation_rotate_90_flip_horizontal_keeps_top_left_in_portrait() {
    let output = convert_jpeg_with_orientation("input.jpg", 5);

    assert_eq!(output.dimensions(), (640, 480));
    assert_portrait_bars(&output);
    assert_white_corner(&output, PORTRAIT_COLUMNS, Corner::TopLeft);
}

#[test]
fn exif_orientation_rotate_90_moves_top_left_to_top_right() {
    let output = convert_jpeg_with_orientation("input.jpg", 6);

    assert_eq!(output.dimensions(), (640, 480));
    assert_portrait_bars(&output);
    assert_white_corner(&output, PORTRAIT_COLUMNS, Corner::TopRight);
}

#[test]
fn exif_orientation_rotate_270_flip_horizontal_moves_top_left_to_bottom_right() {
    let output = convert_jpeg_with_orientation("input.jpg", 7);

    assert_eq!(output.dimensions(), (640, 480));
    assert_portrait_bars(&output);
    assert_white_corner(&output, PORTRAIT_COLUMNS, Corner::BottomRight);
}

#[test]
fn exif_orientation_rotate_270_moves_top_left_to_bottom_left() {
    let output = convert_jpeg_with_orientation("input.jpg", 8);

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
    // ノイズが 0 や 255 で頭打ちにならないよう、中間の灰色にする
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

#[test]
fn broken_image_fails_without_saving() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("input.png"), "not an image").unwrap();

    let output = common::vhsify(tmp.path())
        .arg("input.png")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(common::stderr(&output).contains("Failed to open image: input.png"));
    assert!(common::stdout(&output).is_empty());
    assert_eq!(common::dir_entries(tmp.path()), ["input.png"]);
}

#[test]
fn failed_rename_leaves_no_partial_output() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.png");
    common::write_image(&input, 64, 48);
    // 出力名と同じ名前のフォルダがあると、書き出した画像の名前の変更が失敗する
    std::fs::create_dir(tmp.path().join("input_vhs.png")).unwrap();

    let output = common::vhsify(tmp.path()).arg(&input).output().unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(common::stderr(&output).contains("Failed to save image"));
    assert_eq!(
        common::dir_entries(tmp.path()),
        ["input.png", "input_vhs.png"]
    );
}

#[test]
fn converting_twice_overwrites_the_output() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.png");
    common::write_image(&input, 64, 48);

    let first = common::run(tmp.path(), &input, &[]);
    std::fs::write(&first, "previous output").unwrap();
    let second = common::run(tmp.path(), &input, &[]);

    assert_eq!(second, first);
    assert_eq!(image::image_dimensions(&second).unwrap(), (640, 480));
    assert_eq!(
        common::dir_entries(tmp.path()),
        ["input.png", "input_vhs.png"]
    );
}

#[cfg(unix)]
mod unix {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    #[test]
    fn unwritable_output_dir_fails_without_saving() {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.png");
        common::write_image(&input, 64, 48);
        // 存在の確認は通るが、エフェクトをかけたあとの保存で失敗させる
        let out_dir = tmp.path().join("out");
        std::fs::create_dir(&out_dir).unwrap();
        std::fs::set_permissions(&out_dir, std::fs::Permissions::from_mode(0o555)).unwrap();

        let output = common::vhsify(tmp.path())
            .arg(&input)
            .arg("--output-dir")
            .arg(&out_dir)
            .output()
            .unwrap();

        assert_eq!(output.status.code(), Some(1));
        assert!(common::stderr(&output).contains("Failed to save image"));
        assert!(common::stdout(&output).is_empty());
    }
}

fn assert_format_is_kept(ext: &str) {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join(format!("input.{ext}"));
    common::write_image(&input, 64, 48);

    let output_path = common::run(tmp.path(), &input, &[]);

    assert_eq!(output_path, tmp.path().join(format!("input_vhs.{ext}")));
    assert_eq!(image::image_dimensions(&output_path).unwrap(), (640, 480));
}

/// 左上の 4 分の 1 が白く残りが黒い 640x480 の JPEG に、指定した EXIF の向きを付けて `file_name` という名前で
/// 書き出し、変換する
#[track_caller]
fn convert_jpeg_with_orientation(file_name: &str, orientation: u16) -> GrayImage {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join(file_name);
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

    // 最小限の EXIF APP1 セグメント。ビッグエンディアンの TIFF ヘッダと、Orientation の IFD エントリ 1 つだけ
    let mut app1 = vec![0xFF, 0xE1, 0x00, 0x22];
    app1.extend_from_slice(b"Exif\0\0");
    app1.extend_from_slice(&[b'M', b'M', 0x00, 0x2A, 0x00, 0x00, 0x00, 0x08]);
    app1.extend_from_slice(&[0x00, 0x01]);
    app1.extend_from_slice(&[0x01, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01]);
    app1.extend_from_slice(&orientation.to_be_bytes());
    app1.extend_from_slice(&[0x00, 0x00]);
    app1.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

    // セグメントを SOI マーカーの直後に差し込む
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
    let output_path = common::run(cwd, input, args);
    image::open(output_path).unwrap().into_luma8()
}

#[derive(PartialEq)]
enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// 内容の列のうち、指定した 4 分の 1 だけが白く、残りの 3 つが黒いことを確かめる
#[track_caller]
fn assert_white_corner(img: &GrayImage, content: Range<u32>, white: Corner) {
    // VHS エフェクトで白が黒ににじむので、4 分の 1 の境目からは離れて調べる
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
