pub(crate) const TARGET_WIDTH: u32 = 640;
pub(crate) const TARGET_HEIGHT: u32 = 480;

#[derive(Clone, Copy, clap::ValueEnum)]
pub(crate) enum ScaleMode {
    Bars,
    Crop,
}

/// `width`x`height` の入力を高さ `TARGET_HEIGHT` に縮小したときの幅を、最も近い偶数に丸めて返す。
/// 動画は幅が偶数でなければならない yuv420p で書き出すため。幅が偶数なら余りも偶数になり、
/// 黒帯や切り取りを左右に均等に分けられる
pub(crate) fn scaled_width(width: u32, height: u32) -> u32 {
    // 縮小後の幅は width * TARGET_HEIGHT / height。これを半分にして四捨五入し、2 倍すると
    // 最も近い偶数になる
    let numerator = u64::from(width) * u64::from(TARGET_HEIGHT);
    let denominator = 2 * u64::from(height);
    let pairs = (numerator + denominator / 2) / denominator;
    (pairs * 2) as u32
}

/// 高さ `TARGET_HEIGHT` に縮小した内容を、出力のどこに置くか
#[derive(Debug, PartialEq)]
pub(crate) enum Layout {
    /// 横長の内容を crop モードで: `x` から `TARGET_WIDTH` 列ぶんを残す
    Crop { x: u32 },
    /// 横長の内容を bars モードで: 幅 `width` のまま、左右の `bar_width` 列ずつを黒で覆う
    Bars { width: u32, bar_width: u32 },
    /// 4:3 より横長でない内容: 幅 `TARGET_WIDTH` の黒い画面の `x` の位置に置く
    Pad { x: u32 },
}

impl Layout {
    /// `scaled_width` は [`scaled_width`] で求めた値を想定しているので、偶数になる
    pub(crate) fn new(scaled_width: u32, mode: ScaleMode) -> Self {
        debug_assert!(
            scaled_width % 2 == 0,
            "scaled width must be even, got {scaled_width}"
        );
        if scaled_width <= TARGET_WIDTH {
            return Layout::Pad {
                x: (TARGET_WIDTH - scaled_width) / 2,
            };
        }
        let excess = scaled_width - TARGET_WIDTH;
        match mode {
            ScaleMode::Crop => Layout::Crop { x: excess / 2 },
            ScaleMode::Bars => Layout::Bars {
                width: scaled_width,
                bar_width: excess / 2,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_content_in_bars_mode_keeps_width_and_covers_sides() {
        assert_eq!(
            Layout::new(854, ScaleMode::Bars),
            Layout::Bars {
                width: 854,
                bar_width: 107
            }
        );
    }

    #[test]
    fn wide_content_in_crop_mode_keeps_center_columns() {
        assert_eq!(Layout::new(854, ScaleMode::Crop), Layout::Crop { x: 107 });
    }

    #[test]
    fn content_just_wider_than_4_3_is_cropped_in_crop_mode() {
        // 642 は 640 より大きい最小の偶数
        assert_eq!(Layout::new(642, ScaleMode::Crop), Layout::Crop { x: 1 });
    }

    #[test]
    fn four_by_three_content_fills_the_width_in_any_mode() {
        for mode in [ScaleMode::Bars, ScaleMode::Crop] {
            assert_eq!(Layout::new(640, mode), Layout::Pad { x: 0 });
        }
    }

    #[test]
    fn narrow_content_is_centered_in_any_mode() {
        for mode in [ScaleMode::Bars, ScaleMode::Crop] {
            assert_eq!(Layout::new(360, mode), Layout::Pad { x: 140 });
        }
    }

    #[test]
    fn scaled_width_keeps_the_aspect_ratio_at_480_rows() {
        assert_eq!(scaled_width(640, 480), 640);
        assert_eq!(scaled_width(1280, 960), 640);
        assert_eq!(scaled_width(500, 500), 480);
        assert_eq!(scaled_width(480, 640), 360);
    }

    #[test]
    fn scaled_width_rounds_to_the_nearest_even_number() {
        // 853.3 と 854.7 はどちらも 854、852.7 は 852 になり、ちょうど 853 は切り上がる
        assert_eq!(scaled_width(1920, 1080), 854);
        assert_eq!(scaled_width(1282, 720), 854);
        assert_eq!(scaled_width(1279, 720), 852);
        assert_eq!(scaled_width(853, 480), 854);
    }

    #[test]
    fn wide_content_leaves_exactly_4_3_visible_in_bars_mode() {
        for (src_w, src_h) in [
            (1920, 1080),
            (1282, 720),
            (1279, 720),
            (853, 480),
            (2560, 1080),
        ] {
            let Layout::Bars { width, bar_width } =
                Layout::new(scaled_width(src_w, src_h), ScaleMode::Bars)
            else {
                panic!("expected bars for {src_w}x{src_h}");
            };
            assert_eq!(width - 2 * bar_width, TARGET_WIDTH, "input {src_w}x{src_h}");
        }
    }
}
