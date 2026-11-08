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

/// Returns from the test early when ffmpeg is missing locally; see ffmpeg_available
macro_rules! require_ffmpeg {
    () => {
        if !ffmpeg_available() {
            return;
        }
    };
}

#[test]
fn wide_video_keeps_wide_width_and_covers_sides_with_bars_by_default() {
    require_ffmpeg!();
    for mode_args in [&[][..], &["--mode", "bars"][..]] {
        println!("mode args: {mode_args:?}");
        let tmp = TempDir::new().unwrap();
        let output = convert_video(tmp.path(), "white", 1920, 1080, Audio::Tone, mode_args);

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
    require_ffmpeg!();
    let tmp = TempDir::new().unwrap();
    let output = convert_video(
        tmp.path(),
        "white",
        1920,
        1080,
        Audio::Tone,
        &["--mode", "crop"],
    );

    assert_eq!(probe(&output).size(), (640, 480));
    assert_white(&first_frame(&output), 4..636);
}

#[test]
fn four_by_three_video_has_no_bars_in_any_mode() {
    require_ffmpeg!();
    for mode in ["bars", "crop"] {
        println!("mode: {mode}");
        let tmp = TempDir::new().unwrap();
        let output = convert_video(
            tmp.path(),
            "white",
            1280,
            960,
            Audio::Tone,
            &["--mode", mode],
        );

        assert_eq!(probe(&output).size(), (640, 480));
        assert_white(&first_frame(&output), 4..636);
    }
}

#[test]
fn tall_video_is_padded_with_bars_in_any_mode() {
    require_ffmpeg!();
    for mode in ["bars", "crop"] {
        println!("mode: {mode}");
        let tmp = TempDir::new().unwrap();
        let output = convert_video(
            tmp.path(),
            "white",
            480,
            640,
            Audio::Tone,
            &["--mode", mode],
        );

        assert_eq!(probe(&output).size(), (640, 480));
        let frame = first_frame(&output);
        assert_portrait_bars(&frame);
        assert_white(&frame, 146..494);
    }
}

#[test]
fn square_video_is_padded_with_bars() {
    require_ffmpeg!();
    let tmp = TempDir::new().unwrap();
    let output = convert_video(tmp.path(), "white", 500, 500, Audio::Tone, &[]);

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
    require_ffmpeg!();
    // 1920x1080 gives 853.3 and 1282x720 gives 854.7, so the two rounding paths disagree
    for (width, height) in [(1920, 1080), (1282, 720)] {
        println!("input: {width}x{height}");
        let tmp = TempDir::new().unwrap();
        let video = convert_video(tmp.path(), "white", width, height, Audio::NoTrack, &[]);

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
    require_ffmpeg!();
    let tmp = TempDir::new().unwrap();
    let output = convert_video(tmp.path(), "white", 320, 240, Audio::Tone, &[]);

    let info = probe(&output);
    assert_eq!(info.frame_rate, format!("{FRAME_RATE}/1"));
    assert_eq!(info.frames, Some(5));
}

#[test]
fn audio_is_kept() {
    require_ffmpeg!();
    let tmp = TempDir::new().unwrap();
    let output = convert_video(tmp.path(), "white", 320, 240, Audio::Tone, &[]);

    assert!(probe(&output).has_audio);
}

#[test]
fn video_without_audio_stays_without_audio() {
    require_ffmpeg!();
    let tmp = TempDir::new().unwrap();
    let output = convert_video(tmp.path(), "white", 320, 240, Audio::NoTrack, &[]);

    let info = probe(&output);
    assert_eq!(info.size(), (640, 480));
    assert!(!info.has_audio);
}

#[test]
fn broken_video_fails_without_saving() {
    require_ffmpeg!();
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("input.mp4"), "not a video").unwrap();
    // Point the system temp dir here to check that vhsify cleans up its work dir on error
    let temp_root = tmp.path().join("temp");
    std::fs::create_dir(&temp_root).unwrap();

    let output = common::vhsify(tmp.path())
        .env("TMPDIR", &temp_root)
        .arg("input.mp4")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(common::stderr(&output).contains("Failed to read video info: input.mp4"));
    assert!(common::stdout(&output).is_empty());
    assert!(!tmp.path().join("input_vhs.mp4").exists());
    assert_eq!(std::fs::read_dir(&temp_root).unwrap().count(), 0);
}

#[test]
fn audio_only_file_fails_without_saving() {
    require_ffmpeg!();
    let tmp = TempDir::new().unwrap();
    // ffprobe succeeds but reports no video stream, so the size cannot be read
    run_checked(
        ffmpeg()
            .args(["-f", "lavfi", "-i", "sine=frequency=440:sample_rate=44100"])
            .args(["-t", DURATION, "-y"])
            .arg(tmp.path().join("input.mp4")),
    );

    let output = common::vhsify(tmp.path())
        .arg("input.mp4")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(common::stderr(&output).contains("Failed to read video info: input.mp4"));
    assert!(common::stdout(&output).is_empty());
    assert!(!tmp.path().join("input_vhs.mp4").exists());
}

#[cfg(unix)]
#[test]
fn unwritable_output_dir_fails_without_saving() {
    use std::os::unix::fs::PermissionsExt;

    require_ffmpeg!();
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp4");
    write_video(&input, "white", 320, 240, Audio::NoTrack);
    // Passes the existence check but makes the final encode fail after all frames are processed
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
    assert!(common::stderr(&output).contains("Failed to write video"));
    assert!(common::stdout(&output).is_empty());
}

#[test]
fn missing_ffmpeg_fails_with_message() {
    let tmp = TempDir::new().unwrap();
    // Input validation only checks that the file exists, so an empty file reaches the ffmpeg check
    std::fs::write(tmp.path().join("input.mp4"), "").unwrap();
    let empty_path = tmp.path().join("bin");
    std::fs::create_dir(&empty_path).unwrap();

    let output = common::vhsify(tmp.path())
        .env("PATH", &empty_path)
        .arg("input.mp4")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(common::stderr(&output).contains("ffmpeg not found"));
    assert!(common::stdout(&output).is_empty());
}

// Portrait phone videos are usually stored landscape with a display rotation, which ffmpeg applies when decoding.
// ffprobe reports 270 degrees as -90, so it is checked separately from 90.

#[test]
fn video_rotated_90_degrees_is_padded_as_portrait() {
    require_ffmpeg!();
    assert_rotated_video_is_portrait(90);
}

#[test]
fn video_rotated_270_degrees_is_padded_as_portrait() {
    require_ffmpeg!();
    assert_rotated_video_is_portrait(270);
}

#[test]
fn video_rotated_180_degrees_stays_landscape() {
    require_ffmpeg!();
    let tmp = TempDir::new().unwrap();
    let input = write_rotated_video(tmp.path(), 180);

    let bars = common::run(tmp.path(), &input, &["--mode", "bars"]);
    let info = probe(&bars);
    assert_eq!(info.height, 480);
    assert!(info.width > 640, "expected wide output, got {}", info.width);

    let crop = common::run(tmp.path(), &input, &["--mode", "crop"]);
    assert_eq!(probe(&crop).size(), (640, 480));
    assert_white(&first_frame(&crop), 4..636);
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
    require_ffmpeg!();
    let tmp = TempDir::new().unwrap();
    // Mid gray so that noise is not clipped at 0 or 255
    let output = convert_video(tmp.path(), "0x808080", 640, 480, Audio::NoTrack, &[]);

    let std_dev = std_dev_luma(&first_frame(&output));
    assert!(
        std_dev > NOISE_STD_DEV_MIN,
        "expected noise from the VHS effect, but std dev of luma was {std_dev:.2}"
    );
}

#[track_caller]
fn assert_rotated_video_is_portrait(degrees: u32) {
    for mode in ["bars", "crop"] {
        println!("mode: {mode}");
        let tmp = TempDir::new().unwrap();
        let input = write_rotated_video(tmp.path(), degrees);

        let output = common::run(tmp.path(), &input, &["--mode", mode]);

        // The rotated 1080x1920 frames scale to 270x480 and are centered with 185px bars
        assert_eq!(probe(&output).size(), (640, 480));
        let frame = first_frame(&output);
        assert_black(&frame, 0..179);
        assert_white(&frame, 191..449);
        assert_black(&frame, 461..640);
    }
}

/// Writes a white 1920x1080 video tagged with the given display rotation and returns its path
#[track_caller]
fn write_rotated_video(dir: &Path, degrees: u32) -> PathBuf {
    let landscape = dir.join("landscape.mp4");
    write_video(&landscape, "white", 1920, 1080, Audio::NoTrack);
    let input = dir.join("input.mp4");
    add_rotation(&landscape, &input, degrees);
    input
}

#[track_caller]
fn assert_format_is_kept(ext: &str) {
    require_ffmpeg!();
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join(format!("input.{ext}"));
    write_video(&input, "white", 320, 240, Audio::Tone);

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
    audio: Audio,
    args: &[&str],
) -> PathBuf {
    let input = dir.join("input.mp4");
    write_video(&input, color, width, height, audio);
    common::run(dir, &input, args)
}

enum Audio {
    Tone,
    NoTrack,
}

#[track_caller]
fn write_video(path: &Path, color: &str, width: u32, height: u32, audio: Audio) {
    let mut command = ffmpeg();
    command.args(["-f", "lavfi", "-i"]);
    command.arg(format!("color=c={color}:s={width}x{height}:r={FRAME_RATE}"));
    if let Audio::Tone = audio {
        command.args(["-f", "lavfi", "-i", "sine=frequency=440:sample_rate=44100"]);
    }
    command.args(["-t", DURATION, "-pix_fmt", "yuv420p", "-y"]);
    command.arg(path);
    run_checked(&mut command);
}

#[track_caller]
fn add_rotation(input: &Path, output: &Path, degrees: u32) {
    run_checked(
        ffmpeg()
            .arg("-display_rotation")
            .arg(degrees.to_string())
            .arg("-i")
            .arg(input)
            .args(["-c", "copy", "-y"])
            .arg(output),
    );
}

/// Decodes the first frame through ffmpeg, which converts the limited YUV range to full-range RGB
#[track_caller]
fn first_frame(path: &Path) -> GrayImage {
    let tmp = TempDir::new().unwrap();
    let frame = tmp.path().join("frame.png");
    run_checked(
        ffmpeg()
            .arg("-i")
            .arg(path)
            .args(["-frames:v", "1", "-y"])
            .arg(&frame),
    );
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
    run_checked(
        Command::new("ffprobe")
            .args(["-v", "error", "-of", "default=noprint_wrappers=1"])
            .args(args)
            .arg(path),
    )
}

fn ffmpeg() -> Command {
    let mut command = Command::new("ffmpeg");
    command.args(["-loglevel", "error"]);
    command
}

/// Runs the command, fails the test if it does not succeed, and returns its stdout
#[track_caller]
fn run_checked(command: &mut Command) -> String {
    let output = command.output().unwrap();
    assert!(output.status.success(), "{}", common::stderr(&output));
    common::stdout(&output)
}
