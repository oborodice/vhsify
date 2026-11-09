use std::ops::Range;

use image::GrayImage;

// VHS エフェクトはノイズを加えるので、ピクセル単位ではなく範囲の平均で比べる。
// 実測した平均は、黒帯で約 1、白い内容で約 250
pub const BLACK_MAX: f64 = 20.0;
pub const WHITE_MIN: f64 = 200.0;
// 一様な灰色の画像で実測した輝度の標準偏差は、エフェクトありで約 8、なしで 0
pub const NOISE_STD_DEV_MIN: f64 = 2.0;

#[track_caller]
pub fn assert_portrait_bars(img: &GrayImage) {
    assert_black(img, 0..134);
    assert_black(img, 506..640);
}

#[track_caller]
pub fn assert_black(img: &GrayImage, columns: Range<u32>) {
    assert_black_area(img, columns, 0..img.height());
}

#[track_caller]
pub fn assert_white(img: &GrayImage, columns: Range<u32>) {
    assert_white_area(img, columns, 0..img.height());
}

#[track_caller]
pub fn assert_black_area(img: &GrayImage, columns: Range<u32>, rows: Range<u32>) {
    let mean = mean_luma(img, columns.clone(), rows.clone());
    assert!(
        mean < BLACK_MAX,
        "expected black at columns {columns:?}, rows {rows:?}, but mean luma was {mean:.1}"
    );
}

#[track_caller]
pub fn assert_white_area(img: &GrayImage, columns: Range<u32>, rows: Range<u32>) {
    let mean = mean_luma(img, columns.clone(), rows.clone());
    assert!(
        mean > WHITE_MIN,
        "expected white at columns {columns:?}, rows {rows:?}, but mean luma was {mean:.1}"
    );
}

pub fn mean_luma(img: &GrayImage, columns: Range<u32>, rows: Range<u32>) -> f64 {
    let count = columns.len() * rows.len();
    let sum: f64 = rows
        .flat_map(|y| columns.clone().map(move |x| (x, y)))
        .map(|(x, y)| img.get_pixel(x, y)[0] as f64)
        .sum();
    sum / count as f64
}

pub fn std_dev_luma(img: &GrayImage) -> f64 {
    let mean = mean_luma(img, 0..img.width(), 0..img.height());
    let variance = img
        .pixels()
        .map(|p| (p[0] as f64 - mean).powi(2))
        .sum::<f64>()
        / (img.width() * img.height()) as f64;
    variance.sqrt()
}
