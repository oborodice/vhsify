pub(crate) const TARGET_WIDTH: u32 = 640;
pub(crate) const TARGET_HEIGHT: u32 = 480;

#[derive(Clone, Copy, clap::ValueEnum)]
pub(crate) enum ScaleMode {
    Bars,
    Crop,
}

/// Returns the width of a `width`x`height` source scaled to `TARGET_HEIGHT` rows, rounded to the
/// nearest even number. Videos are encoded as yuv420p, which needs an even width, and an even width
/// also leaves an even excess so that bars and crops split evenly on both sides.
pub(crate) fn scaled_width(width: u32, height: u32) -> u32 {
    // The scaled width is width * TARGET_HEIGHT / height. Halving it, rounding half up and doubling
    // gives the nearest even number.
    let numerator = u64::from(width) * u64::from(TARGET_HEIGHT);
    let denominator = 2 * u64::from(height);
    let pairs = (numerator + denominator / 2) / denominator;
    (pairs * 2) as u32
}

/// Where content scaled to `TARGET_HEIGHT` goes in the output
#[derive(Debug, PartialEq)]
pub(crate) enum Layout {
    /// Wide content in crop mode: keep the `TARGET_WIDTH` columns starting at `x`
    Crop { x: u32 },
    /// Wide content in bars mode: keep the full `width` and cover `bar_width` columns on each side
    Bars { width: u32, bar_width: u32 },
    /// Content no wider than 4:3: place it at `x` on a black `TARGET_WIDTH` canvas
    Pad { x: u32 },
}

impl Layout {
    /// `scaled_width` is expected to come from [`scaled_width`], so it is even
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
        // 642 is the smallest even width above 640
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
        // 853.3 and 854.7 both go to 854, 852.7 goes to 852, and exactly 853 rounds up
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
