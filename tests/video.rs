mod common;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use image::GrayImage;
use tempfile::TempDir;

use common::luma::{
    NOISE_STD_DEV_MIN, assert_black, assert_portrait_bars, assert_white, std_dev_luma,
};

// デバッグビルドでも速く終わるよう、テストの動画は 10 fps で 0.5 秒(5 フレーム)にする
const FRAME_RATE: u32 = 10;
const DURATION: &str = "0.5";

/// 手元に ffmpeg がなければテストをそこで終える。ffmpeg_available を参照
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

        // 正確な幅は画像と異なる。wide_video_has_same_width_as_image を参照
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

    // 500x500 は 480x480 に縮小され、80px の黒帯で中央に置かれる
    assert_eq!(probe(&output).size(), (640, 480));
    let frame = first_frame(&output);
    assert_black(&frame, 0..74);
    assert_white(&frame, 86..554);
    assert_black(&frame, 566..640);
}

#[test]
fn wide_video_has_same_width_as_image() {
    require_ffmpeg!();
    // 1920x1080 は 853.3、1282x720 は 854.7 になるので、切り上げと切り捨ての両方で一致する必要がある
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
    // エラーのときに vhsify が作業用ディレクトリを片付けるかを確かめるため、システムの一時ディレクトリをここに向ける
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
    // ffprobe は成功するが映像のストリームがないので、大きさを読み取れない
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

#[test]
fn failed_rename_leaves_no_partial_output() {
    require_ffmpeg!();
    let case = OutDirCase::new();
    // 出力名と同じ名前のフォルダがあると、書き出した動画の名前の変更が失敗する
    std::fs::create_dir(case.out_dir.join("input_vhs.mp4")).unwrap();

    let output = case.vhsify().output().unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(common::stderr(&output).contains("Failed to write video"));
    assert_eq!(common::dir_entries(&case.out_dir), ["input_vhs.mp4"]);
}

#[test]
fn converting_twice_overwrites_the_output() {
    require_ffmpeg!();
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.mp4");
    write_video(&input, "white", 320, 240, Audio::Tone);

    let first = common::run(tmp.path(), &input, &[]);
    std::fs::write(&first, "previous output").unwrap();
    let second = common::run(tmp.path(), &input, &[]);

    assert_eq!(second, first);
    assert_eq!(probe(&second).size(), (640, 480));
    assert_eq!(
        common::dir_entries(tmp.path()),
        ["input.mp4", "input_vhs.mp4"]
    );
}

#[test]
fn missing_ffmpeg_fails_with_message() {
    let tmp = TempDir::new().unwrap();
    // 入力の検証はファイルがあるかしか見ないので、空のファイルでも ffmpeg の確認まで進む
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

// スマホの縦向きの動画は、ふつう横向きのまま表示用の回転を付けて保存され、ffmpeg がデコード時に回転させる。
// ffprobe は 270 度を -90 と返すので、90 度とは別に確かめる

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
    // ノイズが 0 や 255 で頭打ちにならないよう、中間の灰色にする
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

        // 回転後の 1080x1920 のフレームは 270x480 に縮小され、185px の黒帯で中央に置かれる
        assert_eq!(probe(&output).size(), (640, 480));
        let frame = first_frame(&output);
        assert_black(&frame, 0..179);
        assert_white(&frame, 191..449);
        assert_black(&frame, 461..640);
    }
}

/// 指定した表示用の回転を付けた白い 1920x1080 の動画を書き出し、そのパスを返す
#[track_caller]
fn write_rotated_video(dir: &Path, degrees: u32) -> PathBuf {
    let landscape = dir.join("landscape.mp4");
    write_video(&landscape, "white", 1920, 1080, Audio::NoTrack);
    let input = dir.join("input.mp4");
    add_rotation(&landscape, &input, degrees);
    input
}

/// 320x240 の入力動画と、その変換先になる空の `out` フォルダ
struct OutDirCase {
    tmp: TempDir,
    input: PathBuf,
    out_dir: PathBuf,
}

impl OutDirCase {
    #[track_caller]
    fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp4");
        write_video(&input, "white", 320, 240, Audio::Tone);
        let out_dir = tmp.path().join("out");
        std::fs::create_dir(&out_dir).unwrap();
        OutDirCase {
            tmp,
            input,
            out_dir,
        }
    }

    /// 入力を `out` フォルダへ変換する vhsify のコマンドを返す
    fn vhsify(&self) -> Command {
        let mut command = common::vhsify(self.tmp.path());
        command.arg(&self.input).args(["-d", "out"]);
        command
    }
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

/// 手元に ffmpeg がなければ false を返してテストを飛ばせるようにする。CI では必ず入っているはずなので失敗させる
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

/// 最初のフレームを ffmpeg でデコードする。ffmpeg が limited range の YUV を full range の RGB に変換する
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

/// コマンドを実行し、成功しなければテストを失敗させ、標準出力を返す
#[track_caller]
fn run_checked(command: &mut Command) -> String {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?} exited with {}: {}",
        output.status,
        common::stderr(&output)
    );
    common::stdout(&output)
}

/// Unix に頼るテスト。シグナル、偽の ffmpeg のシェルスクリプト、ファイルの権限を使う
#[cfg(unix)]
mod unix {
    use std::ffi::OsString;
    use std::io::{Read, Seek};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::process::CommandExt;
    use std::process::{ExitStatus, Stdio};
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn unwritable_output_dir_fails_without_saving() {
        require_ffmpeg!();
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp4");
        write_video(&input, "white", 320, 240, Audio::NoTrack);
        // 存在の確認は通るが、全フレームを処理したあとの最後の書き出しで失敗させる
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
    fn interrupting_stops_between_frames_and_removes_the_temp_dir() {
        require_ffmpeg!();
        // 60 フレーム。エフェクトをかけ始めると進捗のファイルが現れる
        let result = interrupt_conversion("2", "progress.txt", "INT", Target::VhsifyOnly);

        assert_eq!(result.status.code(), Some(130));
        assert_eq!(result.leftovers, Vec::<String>::new());
        assert!(result.stderr.contains("Interrupted"));
        // まだ始まっていないフレームは飛ばされるので、最後のフレームまでは処理されない
        assert!(
            !result.stderr.contains("frames 60/60"),
            "all frames were processed: {}",
            result.stderr
        );
    }

    #[test]
    fn interrupting_from_the_terminal_reports_the_interrupt() {
        require_ffmpeg!();
        // 300 フレーム。シグナルが届いたとき、ffmpeg がまだフレームを取り出している最中になるようにする
        let result = interrupt_conversion("10", "frame_00001.jpg", "INT", Target::ProcessGroup);

        assert_eq!(result.status.code(), Some(130));
        assert_eq!(result.leftovers, Vec::<String>::new());
        assert!(result.stderr.contains("Interrupted"), "{}", result.stderr);
        assert!(!result.stderr.contains("Failed to"), "{}", result.stderr);
    }

    #[test]
    fn terminating_stops_between_frames_and_removes_the_temp_dir() {
        require_ffmpeg!();
        // kill コマンドの既定のシグナル
        let result = interrupt_conversion("2", "progress.txt", "TERM", Target::VhsifyOnly);

        // 128 + SIGTERM(15)
        assert_eq!(result.status.code(), Some(143));
        assert_eq!(result.leftovers, Vec::<String>::new());
        assert!(result.stderr.contains("Interrupted"), "{}", result.stderr);
    }

    #[test]
    fn closing_the_terminal_reports_the_interrupt() {
        require_ffmpeg!();
        // ターミナルを閉じると、SIGHUP がプロセスグループ全体に届く
        let result = interrupt_conversion("10", "frame_00001.jpg", "HUP", Target::ProcessGroup);

        // 128 + SIGHUP(1)
        assert_eq!(result.status.code(), Some(129));
        assert_eq!(result.leftovers, Vec::<String>::new());
        assert!(result.stderr.contains("Interrupted"), "{}", result.stderr);
        assert!(!result.stderr.contains("Failed to"), "{}", result.stderr);
    }

    #[test]
    fn failed_frame_extraction_fails_without_saving() {
        require_ffmpeg!();
        let case = OutDirCase::new();
        let fake = fake_ffmpeg(case.tmp.path(), FakeBehavior::FailExtraction);
        // エラーのときに vhsify が作業用ディレクトリを片付けるかを確かめるため、システムの一時ディレクトリをここに向ける
        let temp_root = case.tmp.path().join("temp");
        std::fs::create_dir(&temp_root).unwrap();

        let output = case
            .vhsify()
            .env("PATH", &fake.path)
            .env("TMPDIR", &temp_root)
            .output()
            .unwrap();

        assert_eq!(output.status.code(), Some(1));
        assert!(common::stderr(&output).contains("Failed to extract frames"));
        assert_eq!(common::dir_entries(&case.out_dir), Vec::<String>::new());
        assert_eq!(common::dir_entries(&temp_root), Vec::<String>::new());
    }

    #[test]
    fn failed_write_leaves_no_output() {
        require_ffmpeg!();
        let case = OutDirCase::new();
        let fake = fake_ffmpeg(case.tmp.path(), FakeBehavior::FailWrite);

        let output = case.vhsify().env("PATH", &fake.path).output().unwrap();

        assert_eq!(output.status.code(), Some(1));
        assert!(common::stderr(&output).contains("Failed to write video"));
        assert_eq!(common::dir_entries(&case.out_dir), Vec::<String>::new());
    }

    #[test]
    fn failed_write_keeps_the_existing_output() {
        require_ffmpeg!();
        let case = OutDirCase::new();
        let existing = case.out_dir.join("input_vhs.mp4");
        std::fs::write(&existing, "previous output").unwrap();
        let fake = fake_ffmpeg(case.tmp.path(), FakeBehavior::FailWrite);

        let output = case.vhsify().env("PATH", &fake.path).output().unwrap();

        assert_eq!(output.status.code(), Some(1));
        assert_eq!(common::dir_entries(&case.out_dir), ["input_vhs.mp4"]);
        assert_eq!(
            std::fs::read_to_string(&existing).unwrap(),
            "previous output"
        );
    }

    #[test]
    fn interrupting_from_the_terminal_while_writing_leaves_no_output() {
        require_ffmpeg!();
        let result = interrupt_write("INT", Target::ProcessGroup, WhileWaiting::Release);

        assert_eq!(result.status.code(), Some(130));
        assert!(result.stderr.contains("Interrupted"), "{}", result.stderr);
        assert_eq!(result.leftovers, Vec::<String>::new());
    }

    #[test]
    fn interrupting_while_writing_discards_the_output_even_if_ffmpeg_finishes() {
        require_ffmpeg!();
        let result = interrupt_write("INT", Target::VhsifyOnly, WhileWaiting::Release);

        assert_eq!(result.status.code(), Some(130));
        assert!(result.stderr.contains("Interrupted"), "{}", result.stderr);
        assert_eq!(result.leftovers, Vec::<String>::new());
    }

    #[test]
    fn interrupting_twice_exits_without_waiting_for_ffmpeg() {
        require_ffmpeg!();
        // 偽の ffmpeg は合図がないと書き出しを終えないので、2 回目の SIGINT で終了しない限り vhsify は終わらない
        let result = interrupt_write("INT", Target::VhsifyOnly, WhileWaiting::SignalAgain("INT"));

        assert_eq!(result.status.code(), Some(130));
    }

    #[test]
    fn terminating_while_writing_leaves_no_output() {
        require_ffmpeg!();
        let result = interrupt_write("TERM", Target::VhsifyOnly, WhileWaiting::Release);

        assert_eq!(result.status.code(), Some(143));
        assert!(result.stderr.contains("Interrupted"), "{}", result.stderr);
        assert_eq!(result.leftovers, Vec::<String>::new());
    }

    #[test]
    fn a_different_second_signal_also_exits_without_waiting_for_ffmpeg() {
        require_ffmpeg!();
        // Ctrl+C のあとに kill されたら、後片付けを待たずに、あとのシグナルの終了コードで終わる
        let result = interrupt_write("INT", Target::VhsifyOnly, WhileWaiting::SignalAgain("TERM"));

        assert_eq!(result.status.code(), Some(143));
    }

    /// シグナルの送り先
    enum Target {
        VhsifyOnly,
        /// ターミナルは Ctrl+C などをプロセスグループ全体に送るので、ffmpeg も中断される
        ProcessGroup,
    }

    struct Interrupted {
        status: ExitStatus,
        stderr: String,
        /// vhsify が空にしているはずのフォルダに残ったもの
        leftovers: Vec<String>,
    }

    /// 指定した長さの白い 640x480 30 fps の動画を変換し、vhsify の作業用ディレクトリに `wait_for` という名前の
    /// ファイルが現れたら、`name` のシグナル(`kill -s` に渡す名前)を送る。vhsify の終わり方と、
    /// システムの一時ディレクトリに残ったものを返す
    #[track_caller]
    fn interrupt_conversion(
        seconds: &str,
        wait_for: &str,
        name: &str,
        target: Target,
    ) -> Interrupted {
        let tmp = TempDir::new().unwrap();
        let input = tmp.path().join("input.mp4");
        run_checked(
            ffmpeg()
                .args(["-f", "lavfi", "-i", "color=c=white:s=640x480:r=30"])
                .args(["-t", seconds, "-pix_fmt", "yuv420p", "-y"])
                .arg(&input),
        );
        let temp_root = tmp.path().join("temp");
        std::fs::create_dir(&temp_root).unwrap();

        let mut command = common::vhsify(tmp.path());
        command.env("TMPDIR", &temp_root).arg(&input);
        let ready = || {
            std::fs::read_dir(&temp_root)
                .unwrap()
                .filter_map(|entry| entry.ok())
                .any(|entry| entry.path().join(wait_for).exists())
        };
        let (status, stderr) = interrupt(&mut command, ready, name, target, |_| {});
        Interrupted {
            status,
            stderr,
            leftovers: common::dir_entries(&temp_root),
        }
    }

    /// シグナルを送ってから vhsify が終了するまでにすること
    enum WhileWaiting {
        /// 偽の ffmpeg が中断されていなければ、成功で終わらせる
        Release,
        /// 指定したシグナルを vhsify に送り続ける。1 回目が処理される前に届いた同じシグナルは 1 回にまとめられるため
        SignalAgain(&'static str),
    }

    /// 偽の ffmpeg が出力の動画を書き出している最中に `name` のシグナルを送り、`while_waiting` のとおりにして
    /// 終了を待つ。vhsify の終わり方と、出力先のフォルダに残ったものを返す
    #[track_caller]
    fn interrupt_write(name: &str, target: Target, while_waiting: WhileWaiting) -> Interrupted {
        let case = OutDirCase::new();
        let fake = fake_ffmpeg(case.tmp.path(), FakeBehavior::WaitForRelease);

        let mut command = case.vhsify();
        command.env("PATH", &fake.path);
        let ready = || !common::dir_entries(&case.out_dir).is_empty();
        let act = |pid| match while_waiting {
            WhileWaiting::Release => std::fs::write(&fake.release, "").unwrap(),
            WhileWaiting::SignalAgain(again) => send_signal(pid, again, Target::VhsifyOnly),
        };
        let (status, stderr) = interrupt(&mut command, ready, name, target, act);
        Interrupted {
            status,
            stderr,
            leftovers: common::dir_entries(&case.out_dir),
        }
    }

    /// `command` を専用のプロセスグループで起動し、`ready` が true を返したら `name` のシグナルを送る。そのあと
    /// 終了するまで、100 ms ごとに pid を渡して `while_waiting` を呼ぶ。終了状態と標準エラー出力を返す
    #[track_caller]
    fn interrupt(
        command: &mut Command,
        ready: impl Fn() -> bool,
        name: &str,
        target: Target,
        mut while_waiting: impl FnMut(u32),
    ) -> (ExitStatus, String) {
        // パイプがいっぱいにならないよう、標準エラー出力はファイルに書かせて、あとから進捗を読む
        let mut stderr = tempfile::tempfile().unwrap();
        let mut child = command
            .process_group(0)
            .stdout(Stdio::null())
            .stderr(stderr.try_clone().unwrap())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(60);
        while !ready() {
            assert!(
                Instant::now() < deadline,
                "never got ready to send SIG{name}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        send_signal(child.id(), name, target);

        let status = loop {
            while_waiting(child.id());
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                panic!("did not exit after SIG{name}");
            }
            std::thread::sleep(Duration::from_millis(100));
        };
        let mut text = String::new();
        stderr.rewind().unwrap();
        stderr.read_to_string(&mut text).unwrap();
        (status, text)
    }

    #[track_caller]
    fn send_signal(pid: u32, name: &str, target: Target) {
        let target = match target {
            Target::VhsifyOnly => pid.to_string(),
            Target::ProcessGroup => format!("-{pid}"),
        };
        // 負のプロセスグループ ID が Linux の kill にオプションとして読まれないよう、`--` を付ける
        run_checked(Command::new("kill").args(["-s", name, "--", &target]));
    }

    /// 偽の ffmpeg がどうふるまうか
    enum FakeBehavior {
        /// 入力の動画からフレームを取り出す呼び出しで失敗する
        FailExtraction,
        /// 出力の動画を途中まで書いて失敗する
        FailWrite,
        /// 出力の動画を途中まで書き、テストが [`FakeFfmpeg`] の `release` のファイルを作ったら成功する
        WaitForRelease,
    }

    struct FakeFfmpeg {
        /// 本物の ffmpeg より先に偽物が見つかる PATH
        path: OsString,
        release: PathBuf,
    }

    /// 偽の ffmpeg を `dir/bin` に書き出す。偽物は `behavior` で決めた呼び出し以外をすべて本物の ffmpeg に渡すので、
    /// その呼び出しがどう終わるかをテストで決められる
    #[track_caller]
    fn fake_ffmpeg(dir: &Path, behavior: FakeBehavior) -> FakeFfmpeg {
        let real = run_checked(Command::new("sh").args(["-c", "command -v ffmpeg"]));
        let bin = dir.join("bin");
        std::fs::create_dir(&bin).unwrap();
        let release = bin.join("release");
        let real = real.trim();
        let write = |finish: &str| {
            format!(
                r#"#!/bin/sh
# 引数が -r で始まるのは、出力の動画を書き出す呼び出しだけ
if [ "$1" != "-r" ]; then exec '{real}' "$@"; fi
# 出力のパスは、最後の -y の直前にある
for arg; do output=$previous; previous=$arg; done
echo partial > "$output"
{finish}
"#
            )
        };
        let script = match behavior {
            FakeBehavior::FailExtraction => format!(
                r#"#!/bin/sh
# -vf を含むのは、入力の動画からフレームを取り出す呼び出しだけ
for arg; do if [ "$arg" = "-vf" ]; then exit 1; fi; done
exec '{real}' "$@"
"#
            ),
            FakeBehavior::FailWrite => write("exit 1"),
            // vhsify が先に終了して偽の ffmpeg が取り残されても動き続けないよう、テストの一時フォルダが消えたら終える
            FakeBehavior::WaitForRelease => write(&format!(
                "while [ ! -e '{}' ] && [ -d '{}' ]; do sleep 0.05; done",
                release.display(),
                bin.display()
            )),
        };
        let fake = bin.join("ffmpeg");
        std::fs::write(&fake, script).unwrap();
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();

        let path = std::env::var_os("PATH").unwrap_or_default();
        let path =
            std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(&path))).unwrap();
        FakeFfmpeg { path, release }
    }
}
