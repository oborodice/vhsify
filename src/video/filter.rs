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
