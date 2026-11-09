use std::ffi::OsStr;
use std::process::Command;

/// ffmpeg が見つからなければエラーを返す
pub(crate) fn check_installed() -> Result<(), String> {
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        return Err("ffmpeg not found. Please install ffmpeg and try again.".to_string());
    }
    Ok(())
}

/// ffmpeg を実行し、成功したかを返す。失敗が何を意味するかは呼び出し側が決める
pub(crate) fn run<S: AsRef<OsStr>>(args: &[S]) -> bool {
    Command::new("ffmpeg")
        .args(args)
        .status()
        .expect("Failed to run ffmpeg")
        .success()
}

/// ffprobe を実行し、成功したら標準出力を返す
pub(crate) fn probe<S: AsRef<OsStr>>(args: &[S]) -> Option<String> {
    let output = Command::new("ffprobe")
        .args(args)
        .output()
        .expect("Failed to run ffprobe");
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// ffmpeg が macOS のハードウェアエンコーダ(VideoToolbox)を使えるか
pub(crate) fn has_videotoolbox() -> bool {
    Command::new("ffmpeg")
        .args(["-encoders", "-v", "quiet"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("h264_videotoolbox"))
        .unwrap_or(false)
}
