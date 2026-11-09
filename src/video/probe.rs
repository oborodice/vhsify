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

#[cfg(test)]
mod tests {
    use super::*;

    fn info(width: u32, height: u32, fps: &str) -> Option<VideoInfo> {
        Some(VideoInfo {
            width,
            height,
            fps: fps.to_string(),
        })
    }

    #[test]
    fn reads_the_size_and_frame_rate() {
        let output = "width=1920\nheight=1080\nr_frame_rate=30000/1001\n";

        assert_eq!(parse(output), info(1920, 1080, "30000/1001"));
    }

    #[test]
    fn quarter_turns_swap_width_and_height() {
        // 時計回りか反時計回りかで、ffprobe は 90 と -90(270)のどちらも返す
        for rotation in ["90", "-90", "270", "-270"] {
            let output = format!("width=320\nheight=240\nr_frame_rate=30/1\nrotation={rotation}\n");

            assert_eq!(
                parse(&output),
                info(240, 320, "30/1"),
                "rotation={rotation}"
            );
        }
    }

    #[test]
    fn half_and_full_turns_keep_width_and_height() {
        for rotation in ["0", "180", "-180", "360"] {
            let output = format!("width=320\nheight=240\nr_frame_rate=30/1\nrotation={rotation}\n");

            assert_eq!(
                parse(&output),
                info(320, 240, "30/1"),
                "rotation={rotation}"
            );
        }
    }

    #[test]
    fn rotation_close_to_a_quarter_turn_is_rounded() {
        let output = "width=320\nheight=240\nr_frame_rate=30/1\nrotation=89.999\n";

        assert_eq!(parse(output), info(240, 320, "30/1"));
    }

    #[test]
    fn unreadable_rotation_is_ignored() {
        let output = "width=320\nheight=240\nr_frame_rate=30/1\nrotation=N/A\n";

        assert_eq!(parse(output), info(320, 240, "30/1"));
    }

    #[test]
    fn missing_or_unreadable_size_or_frame_rate_is_none() {
        for output in [
            "height=240\nr_frame_rate=30/1\n",
            "width=320\nr_frame_rate=30/1\n",
            "width=320\nheight=240\n",
            "width=N/A\nheight=240\nr_frame_rate=30/1\n",
            "",
        ] {
            assert_eq!(parse(output), None, "{output:?}");
        }
    }
}
