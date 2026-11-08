mod common;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use image::GrayImage;
use tempfile::TempDir;

use common::luma::{
    NOISE_STD_DEV_MIN, assert_black, assert_portrait_bars, assert_white, std_dev_luma,
};

// Test videos are 0.5 seconds at 10 fps, i.e. 5 frames, to keep the debug build fast
const FRAME_RATE: u32 = 10;
const DURATION: &str = "0.5";

#[test]
fn wide_video_keeps_wide_width_and_covers_sides_with_bars_by_default() {
    if !ffmpeg_available() {
        return;
    }
    for mode_args in [&[][..], &["--mode", "bars"][..]] {
        println!("mode args: {mode_args:?}");
        let tmp = TempDir::new().unwrap();
        let output = convert_video(tmp.path(), "white", 1920, 1080, true, mode_args);

        // The exact width differs from images; see wide_video_has_same_width_as_image
        let info = probe(&output);
        assert_eq!(info.height, 480);
        assert!(info.width > 640, "expected wide output, got {}", info.width);

        let frame = first_frame(&output);
        assert_black(&frame, 0..100);
        assert_white(&frame, 112..741);
        assert_black(&frame, 753..frame.width());
    }
}

#[test]
fn wide_video_is_cropped_to_4_3_in_crop_mode() {
    if !ffmpeg_available() {
        return;
    }
    let tmp = TempDir::new().unwrap();
    let output = convert_video(tmp.path(), "white", 1920, 1080, true, &["--mode", "crop"]);

    assert_eq!(probe(&output).size(), (640, 480));
    assert_white(&first_frame(&output), 4..636);
}

#[test]
fn four_by_three_video_has_no_bars_in_any_mode() {
    if !ffmpeg_available() {
        return;
    }
    for mode in ["bars", "crop"] {
        println!("mode: {mode}");
        let tmp = TempDir::new().unwrap();
        let output = convert_video(tmp.path(), "white", 1280, 960, true, &["--mode", mode]);

        assert_eq!(probe(&output).size(), (640, 480));
        assert_white(&first_frame(&output), 4..636);
    }
}

#[test]
fn tall_video_is_padded_with_bars_in_any_mode() {
    if !ffmpeg_available() {
        return;
    }
    for mode in ["bars", "crop"] {
        println!("mode: {mode}");
        let tmp = TempDir::new().unwrap();
        let output = convert_video(tmp.path(), "white", 480, 640, true, &["--mode", mode]);

        assert_eq!(probe(&output).size(), (640, 480));
        let frame = first_frame(&output);
        assert_portrait_bars(&frame);
        assert_white(&frame, 146..494);
    }
}

#[test]
fn square_video_is_padded_with_bars() {
    if !ffmpeg_available() {
        return;
    }
    let tmp = TempDir::new().unwrap();
    let output = convert_video(tmp.path(), "white", 500, 500, true, &[]);

    // 500x500 scales to 480x480 and is centered with 80px bars
    assert_eq!(probe(&output).size(), (640, 480));
    let frame = first_frame(&output);
    assert_black(&frame, 0..74);
    assert_white(&frame, 86..554);
    assert_black(&frame, 566..640);
}

#[test]
#[ignore = "known bug: video width is truncated and rounded to even by ffmpeg, while image width is rounded"]
fn wide_video_has_same_width_as_image() {
    if !ffmpeg_available() {
        return;
    }
    // 1920x1080 gives 853.3 and 1282x720 gives 854.7, so the two rounding paths disagree
    for (width, height) in [(1920, 1080), (1282, 720)] {
        println!("input: {width}x{height}");
        let tmp = TempDir::new().unwrap();
        let video = convert_video(tmp.path(), "white", width, height, false, &[]);

        let image_input = tmp.path().join("input.png");
        common::write_image(&image_input, width, height);
        let image = common::run(tmp.path(), &image_input, &[]);

        assert_eq!(
            probe(&video).width,
            image::image_dimensions(&image).unwrap().0
        );
    }
}

#[test]
fn frame_rate_and_frame_count_are_kept() {
    if !ffmpeg_available() {
        return;
    }
    let tmp = TempDir::new().unwrap();
    let output = convert_video(tmp.path(), "white", 320, 240, true, &[]);

    let info = probe(&output);
    assert_eq!(info.frame_rate, format!("{FRAME_RATE}/1"));
    assert_eq!(info.frames, Some(5));
}

#[test]
fn audio_is_kept() {
    if !ffmpeg_available() {
        return;
    }
    let tmp = TempDir::new().unwrap();
    let output = convert_video(tmp.path(), "white", 320, 240, true, &[]);

    assert!(probe(&output).has_audio);
}

#[test]
fn video_without_audio_stays_without_audio() {
    if !ffmpeg_available() {
        return;
    }
    let tmp = TempDir::new().unwrap();
    let output = convert_video(tmp.path(), "white", 320, 240, false, &[]);

    let info = probe(&output);
    assert_eq!(info.size(), (640, 480));
    assert!(!info.has_audio);
}

#[test]
fn broken_video_fails_without_saving() {
    if !ffmpeg_available() {
        return;
    }
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("input.mp4"), "not a video").unwrap();

    let output = common::vhsify(tmp.path())
        .arg("input.mp4")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(common::stderr(&output).contains("Failed to read video info: input.mp4"));
    assert!(common::stdout(&output).is_empty());
    assert!(!tmp.path().join("input_vhs.mp4").exists());
}

#[test]
fn mp4_input_is_saved_as_mp4() {
    assert_format_is_kept("mp4");
}

#[test]
fn mov_input_is_saved_as_mov() {
    assert_format_is_kept("mov");
}

#[test]
fn avi_input_is_saved_as_avi() {
    assert_format_is_kept("avi");
}

#[test]
fn mkv_input_is_saved_as_mkv() {
    assert_format_is_kept("mkv");
}

#[test]
fn vhs_effect_adds_noise_to_flat_video() {
    if !ffmpeg_available() {
        return;
    }
    let tmp = TempDir::new().unwrap();
    // Mid gray so that noise is not clipped at 0 or 255
    let output = convert_video(tmp.path(), "0x808080", 640, 480, false, &[]);

    let std_dev = std_dev_luma(&first_frame(&output));
    assert!(
        std_dev > NOISE_STD_DEV_MIN,
        "expected noise from the VHS effect, but std dev of luma was {std_dev:.2}"
    );
}

#[track_caller]
fn assert_format_is_kept(ext: &str) {
    if !ffmpeg_available() {
        return;
    }
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join(format!("input.{ext}"));
    write_video(&input, "white", 320, 240, true);

    let output = common::run(tmp.path(), &input, &[]);

    assert_eq!(output, tmp.path().join(format!("input_vhs.{ext}")));
    assert_eq!(probe(&output).size(), (640, 480));
}

/// Returns false so the test can skip when ffmpeg is missing locally, but fails in CI where it must be installed
fn ffmpeg_available() -> bool {
    let found = ["ffmpeg", "ffprobe"]
        .iter()
        .all(|tool| Command::new(tool).arg("-version").output().is_ok());
    if !found {
        assert!(
            std::env::var_os("CI").is_none(),
            "ffmpeg and ffprobe are required in CI"
        );
        eprintln!("skipping: ffmpeg or ffprobe not found");
    }
    found
}

#[track_caller]
fn convert_video(
    dir: &Path,
    color: &str,
    width: u32,
    height: u32,
    with_audio: bool,
    args: &[&str],
) -> PathBuf {
    let input = dir.join("input.mp4");
    write_video(&input, color, width, height, with_audio);
    common::run(dir, &input, args)
}

#[track_caller]
fn write_video(path: &Path, color: &str, width: u32, height: u32, with_audio: bool) {
    let mut command = Command::new("ffmpeg");
    command.args(["-loglevel", "error", "-f", "lavfi", "-i"]);
    command.arg(format!("color=c={color}:s={width}x{height}:r={FRAME_RATE}"));
    if with_audio {
        command.args(["-f", "lavfi", "-i", "sine=frequency=440:sample_rate=44100"]);
    }
    command.args(["-t", DURATION, "-pix_fmt", "yuv420p", "-y"]);
    command.arg(path);

    let output = command.output().unwrap();
    assert!(output.status.success(), "{}", common::stderr(&output));
}

/// Decodes the first frame through ffmpeg, which converts the limited YUV range to full-range RGB
#[track_caller]
fn first_frame(path: &Path) -> GrayImage {
    let tmp = TempDir::new().unwrap();
    let frame = tmp.path().join("frame.png");
    let output = Command::new("ffmpeg")
        .args(["-loglevel", "error", "-i"])
        .arg(path)
        .args(["-frames:v", "1", "-y"])
        .arg(&frame)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", common::stderr(&output));
    image::open(&frame).unwrap().into_luma8()
}

struct VideoInfo {
    width: u32,
    height: u32,
    frame_rate: String,
    frames: Option<u32>,
    has_audio: bool,
}

impl VideoInfo {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

#[track_caller]
fn probe(path: &Path) -> VideoInfo {
    let video = ffprobe(
        path,
        &[
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,r_frame_rate,nb_frames",
        ],
    );
    let fields: HashMap<&str, &str> = video
        .lines()
        .filter_map(|line| line.split_once('='))
        .collect();
    let audio = ffprobe(
        path,
        &["-select_streams", "a", "-show_entries", "stream=index"],
    );

    VideoInfo {
        width: fields["width"].parse().unwrap(),
        height: fields["height"].parse().unwrap(),
        frame_rate: fields["r_frame_rate"].to_string(),
        frames: fields["nb_frames"].parse().ok(),
        has_audio: !audio.trim().is_empty(),
    }
}

#[track_caller]
fn ffprobe(path: &Path, args: &[&str]) -> String {
    let output = Command::new("ffprobe")
        .args(["-v", "error", "-of", "default=noprint_wrappers=1"])
        .args(args)
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", common::stderr(&output));
    common::stdout(&output)
}
