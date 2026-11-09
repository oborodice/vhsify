use crate::layout::{Layout, ScaleMode, TARGET_HEIGHT, TARGET_WIDTH};

/// 幅 `scaled_w` に縮小した映像を、`scale_mode` の配置で 640x480 にする ffmpeg の映像フィルタ(`-vf` の値)を返す
pub(super) fn video_filter(scaled_w: u32, scale_mode: ScaleMode) -> String {
    let scale = format!("scale={scaled_w}:{TARGET_HEIGHT}");
    match Layout::new(scaled_w, scale_mode) {
        Layout::Crop { x } => format!("{scale},crop={TARGET_WIDTH}:{TARGET_HEIGHT}:{x}:0"),
        Layout::Bars { width, bar_width } => {
            let bar = |x: u32| {
                format!("drawbox=x={x}:y=0:w={bar_width}:h={TARGET_HEIGHT}:color=black:t=fill")
            };
            format!("{scale},{},{}", bar(0), bar(width - bar_width))
        }
        Layout::Pad { x } => format!("{scale},pad={TARGET_WIDTH}:{TARGET_HEIGHT}:{x}:0:black"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_content_in_bars_mode_keeps_its_width_and_draws_a_bar_on_each_side() {
        // 854 列のうち、左右の 107 列ずつを覆う
        assert_eq!(
            video_filter(854, ScaleMode::Bars),
            "scale=854:480,\
             drawbox=x=0:y=0:w=107:h=480:color=black:t=fill,\
             drawbox=x=747:y=0:w=107:h=480:color=black:t=fill"
        );
    }

    #[test]
    fn wide_content_in_crop_mode_keeps_the_center_columns() {
        assert_eq!(
            video_filter(854, ScaleMode::Crop),
            "scale=854:480,crop=640:480:107:0"
        );
    }

    #[test]
    fn narrow_content_is_padded_to_the_center_in_any_mode() {
        for mode in [ScaleMode::Bars, ScaleMode::Crop] {
            assert_eq!(
                video_filter(360, mode),
                "scale=360:480,pad=640:480:140:0:black"
            );
        }
    }

    #[test]
    fn four_by_three_content_is_padded_without_a_margin_in_any_mode() {
        for mode in [ScaleMode::Bars, ScaleMode::Crop] {
            assert_eq!(
                video_filter(640, mode),
                "scale=640:480,pad=640:480:0:0:black"
            );
        }
    }
}
