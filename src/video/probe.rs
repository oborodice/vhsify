use std::collections::HashMap;

/// 動画の最初の映像ストリームの情報
#[derive(Debug, PartialEq)]
pub(super) struct VideoInfo {
    /// 回転を反映したあとの幅
    pub(super) width: u32,
    /// 回転を反映したあとの高さ
    pub(super) height: u32,
    /// ffprobe が返したままのフレームレート(`30000/1001` など)。ffmpeg の `-r` にそのまま渡す
    pub(super) fps: String,
}

/// `ffprobe -show_entries stream=width,height,r_frame_rate:stream_side_data=rotation
/// -of default=noprint_wrappers=1` の出力を読む。大きさかフレームレートが読めなければ None を返す
pub(super) fn parse(output: &str) -> Option<VideoInfo> {
    let fields: HashMap<&str, &str> = output
        .lines()
        .filter_map(|line| line.split_once('='))
        .collect();
    let size = |key| fields.get(key).and_then(|val| val.parse::<u32>().ok());
    let (width, height) = (size("width")?, size("height")?);
    let fps = fields.get("r_frame_rate")?.to_string();
    let rotation: f64 = fields
        .get("rotation")
        .and_then(|val| val.parse().ok())
        .unwrap_or(0.0);

    // ffprobe は保存されたままの大きさを返すが、ffmpeg はデコードしたフレームを回転させるので、90 度単位の回転では縦横が入れ替わる
    let quarter_turn = matches!((rotation.round() as i64).rem_euclid(360), 90 | 270);
    let (width, height) = if quarter_turn {
        (height, width)
    } else {
        (width, height)
    };
    Some(VideoInfo { width, height, fps })
}
