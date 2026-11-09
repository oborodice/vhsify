mod filter;
mod probe;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::ffmpeg;
use crate::image as vhs_image;
use crate::interrupt;
use crate::layout::{self, ScaleMode};
use probe::VideoInfo;

pub(crate) fn process(
    input_path: &str,
    scale_mode: ScaleMode,
    output_dir: Option<&str>,
    output_name: Option<&str>,
) -> Result<String, String> {
    ffmpeg::check_installed()?;
    init_thread_pool();
    let temp = create_temp_dir();
    let frame_pattern_str = temp.frame_pattern.to_str().unwrap();

    let info = get_video_info(input_path)?;
    extract_frames(
        input_path,
        frame_pattern_str,
        info.width,
        info.height,
        scale_mode,
    )?;
    interrupt::check()?;

    let frames = collect_frames(&temp.dir);
    if frames.is_empty() {
        return Err(format!("No video frames found: {}", input_path));
    }
    apply_effects_to_frames(&frames, &temp.progress);
    interrupt::check()?;

    let has_audio = process_audio(input_path, &temp.raw_wav, &temp.processed_wav)?;
    interrupt::check()?;
    let output_path = crate::output_path::build(input_path, output_dir, output_name);
    let audio_path = has_audio.then(|| temp.processed_wav.to_str().unwrap().to_string());
    reassemble(
        frame_pattern_str,
        &info.fps,
        &output_path,
        audio_path.as_deref(),
    )?;

    Ok(output_path)
}

struct TempDir {
    frame_pattern: PathBuf,
    raw_wav: PathBuf,
    processed_wav: PathBuf,
    progress: PathBuf,
    dir: PathBuf,
}

// エラーや panic で処理が中断されたときも一時ディレクトリを削除する
impl Drop for TempDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

fn init_thread_pool() {
    rayon::ThreadPoolBuilder::new()
        .stack_size(vhs_image::EFFECT_STACK_SIZE)
        .build_global()
        .ok();
}

fn create_temp_dir() -> TempDir {
    let dir = std::env::temp_dir().join(format!("vhsify_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("Failed to create temp dir");
    TempDir {
        frame_pattern: dir.join("frame_%05d.jpg"),
        raw_wav: dir.join("audio_raw.wav"),
        processed_wav: dir.join("audio_vhs.wav"),
        progress: dir.join("progress.txt"),
        dir,
    }
}

fn get_video_info(input_path: &str) -> Result<VideoInfo, String> {
    let error = || format!("Failed to read video info: {}", input_path);
    let mut args = probe::ARGS.to_vec();
    args.push(input_path);
    let output = ffmpeg::probe(&args).ok_or_else(error)?;
    probe::parse(&output).ok_or_else(error)
}

fn extract_frames(
    input_path: &str,
    frame_pattern: &str,
    src_w: u32,
    src_h: u32,
    scale_mode: ScaleMode,
) -> Result<(), String> {
    let scaled_w = layout::scaled_width(src_w, src_h);
    let video_filter = filter::video_filter(scaled_w, scale_mode);
    if !ffmpeg::run(&["-i", input_path, "-vf", &video_filter, frame_pattern, "-y"]) {
        return Err(format!("Failed to extract frames: {}", input_path));
    }
    Ok(())
}

fn collect_frames(temp_dir: &Path) -> Vec<PathBuf> {
    let mut frames: Vec<_> = std::fs::read_dir(temp_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "jpg").unwrap_or(false))
        .collect();
    frames.sort();
    frames
}

fn apply_effects_to_frames(frames: &[PathBuf], progress_path: &Path) {
    let total: usize = frames.len();
    let counter = AtomicUsize::new(0);
    frames.par_iter().enumerate().for_each(|(i, frame_path)| {
        if interrupt::requested() {
            return;
        }
        apply_effect_to_frame(frame_path, i);
        let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
        report_progress(done, total, progress_path);
    });
    eprintln!();
    let _ = std::fs::write(progress_path, "audio processing...\n");
}

fn process_audio(input_path: &str, raw_wav: &Path, processed_wav: &Path) -> Result<bool, String> {
    let has_audio = crate::audio::extract(input_path, raw_wav.to_str().unwrap());
    if has_audio {
        eprintln!("Processing audio...");
        crate::audio::apply_effects(raw_wav.to_str().unwrap(), processed_wav.to_str().unwrap())?;
    }
    Ok(has_audio)
}

fn reassemble(
    frame_pattern: &str,
    fps: &str,
    output_path: &str,
    audio_path: Option<&str>,
) -> Result<(), String> {
    let mut args = vec![
        "-r".to_string(),
        fps.to_string(),
        "-i".to_string(),
        frame_pattern.to_string(),
    ];
    args.extend(audio_args(audio_path));
    args.extend(encoder_args());
    args.extend(["-pix_fmt".to_string(), "yuv420p".to_string()]);

    let error = format!("Failed to write video: {}", output_path);
    crate::output_path::write_via_partial(output_path, error.clone(), |partial| {
        args.extend([partial.to_string_lossy().into_owned(), "-y".to_string()]);
        if !ffmpeg::run(&args) {
            return Err(error);
        }
        // ffmpeg が書き終えていても、中断されたなら保存しない
        interrupt::check()
    })
}

fn apply_effect_to_frame(frame_path: &Path, frame_index: usize) {
    let frame_str = frame_path.to_str().unwrap();
    let mut rgb = image::open(frame_str)
        .expect("Failed to open frame")
        .into_rgb8();
    vhs_image::apply_effect(&mut rgb, frame_index);
    rgb.save(frame_str).expect("Failed to save frame");
}

fn report_progress(done: usize, total: usize, progress_path: &Path) {
    let percentage = done * 100 / total;
    if done % (total / 20).max(1) == 0 || done == total {
        let status = format!("frames {}/{} ({}%)\n", done, total, percentage);
        let _ = std::fs::write(progress_path, &status);
        eprint!("\r{}", status.trim());
    }
}

fn audio_args(audio_path: Option<&str>) -> Vec<String> {
    if let Some(audio) = audio_path {
        vec![
            "-i".to_string(),
            audio.to_string(),
            "-map".to_string(),
            "0:v".to_string(),
            "-map".to_string(),
            "1:a".to_string(),
            "-c:a".to_string(),
            "aac".to_string(),
        ]
    } else {
        vec!["-map".to_string(), "0:v".to_string()]
    }
}

fn encoder_args() -> Vec<String> {
    if ffmpeg::has_videotoolbox() {
        vec!["-c:v".to_string(), "h264_videotoolbox".to_string()]
    } else {
        vec!["-preset".to_string(), "ultrafast".to_string()]
    }
}
