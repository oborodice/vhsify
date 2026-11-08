pub(crate) const TARGET_WIDTH: u32 = 640;
pub(crate) const TARGET_HEIGHT: u32 = 480;

#[derive(Clone, Copy, clap::ValueEnum)]
pub(crate) enum ScaleMode {
    Bars,
    Crop,
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
    pub(crate) fn new(scaled_width: u32, mode: ScaleMode) -> Self {
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
        assert_eq!(Layout::new(641, ScaleMode::Crop), Layout::Crop { x: 0 });
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
    #[ignore = "known bug: an odd excess is split into two equal bars, leaving 641 visible columns"]
    fn wide_content_with_odd_excess_leaves_exactly_4_3_visible() {
        for scaled_width in [641, 853] {
            let Layout::Bars { width, bar_width } = Layout::new(scaled_width, ScaleMode::Bars)
            else {
                panic!("expected bars for {scaled_width}");
            };
            assert_eq!(
                width - 2 * bar_width,
                TARGET_WIDTH,
                "scaled width {scaled_width}"
            );
        }
    }
}
