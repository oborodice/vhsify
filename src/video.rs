use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::image as vhs_image;

pub fn process(input_path: &str, scale_mode: crate::utils::ScaleMode, output_dir: Option<&str>, output_name: Option<&str>) -> String {
    init_thread_pool();
    let temp = create_temp_dir();
    let frame_pattern_str = temp.frame_pattern.to_str().unwrap();

    let (src_w, src_h, fps) = get_video_info(input_path);
    extract_frames(input_path, frame_pattern_str, src_w, src_h, scale_mode);

    let frames = collect_frames(&temp.dir);
    apply_effects_to_frames(&frames, &temp.progress);

    let has_audio = process_audio(input_path, &temp.raw_wav, &temp.processed_wav);
    let output_path = crate::utils::make_output_path(input_path, output_dir, output_name);
    let audio_path = has_audio.then(|| temp.processed_wav.to_str().unwrap().to_string());
    reassemble(frame_pattern_str, &fps, &output_path, audio_path.as_deref());

    std::fs::remove_dir_all(&temp.dir).ok();

    output_path
}

struct TempPaths {
    frame_pattern: PathBuf,
    raw_wav: PathBuf,
    processed_wav: PathBuf,
    progress: PathBuf,
    dir: PathBuf,
}

fn init_thread_pool() {
    rayon::ThreadPoolBuilder::new()
        .stack_size(8 * 1024 * 1024)
        .build_global()
        .ok();
}

fn create_temp_dir() -> TempPaths {
    let dir = std::env::temp_dir().join(format!("vhsify_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("Failed to create temp dir");
    TempPaths {
        frame_pattern: dir.join("frame_%05d.jpg"),
        raw_wav: dir.join("audio_raw.wav"),
        processed_wav: dir.join("audio_vhs.wav"),
        progress: dir.join("progress.txt"),
        dir,
    }
}

fn get_video_info(input_path: &str) -> (u32, u32, String) {
    let output = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=width,height,r_frame_rate",
            "-of", "csv=p=0",
            input_path,
        ])
        .output()
        .expect("Failed to run ffprobe");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.trim().trim_end_matches(',');
    let mut parts = line.split(',');
    let width = parts.next().and_then(|val| val.parse().ok()).unwrap_or(640);
    let height = parts.next().and_then(|val| val.parse().ok()).unwrap_or(480);
    let fps = parts.next().unwrap_or("30").to_string();
    (width, height, fps)
}

fn extract_frames(input_path: &str, frame_pattern: &str, src_w: u32, src_h: u32, scale_mode: crate::utils::ScaleMode) {
    let (visible_w, target_h) = (640u32, 480u32);
    let scaled_w = src_w * target_h / src_h;
    let video_filter = if scaled_w > visible_w && matches!(scale_mode, crate::utils::ScaleMode::Crop) {
        video_filter_crop(scaled_w, visible_w, target_h)
    } else {
        video_filter_bars(scaled_w, visible_w, target_h)
    };
    Command::new("ffmpeg")
        .args(["-i", input_path, "-vf", &video_filter, frame_pattern, "-y"])
        .status()
        .expect("Failed to run ffmpeg");
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
        apply_effect_to_frame(frame_path, i);
        let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
        report_progress(done, total, progress_path);
    });
    eprintln!();
    let _ = std::fs::write(progress_path, "audio processing...\n");
}

fn process_audio(input_path: &str, raw_wav: &Path, processed_wav: &Path) -> bool {
    let has_audio = crate::audio::extract(input_path, raw_wav.to_str().unwrap());
    if has_audio {
        eprintln!("Processing audio...");
        crate::audio::apply_effects(raw_wav.to_str().unwrap(), processed_wav.to_str().unwrap());
    }
    has_audio
}

fn reassemble(frame_pattern: &str, fps: &str, output_path: &str, audio_path: Option<&str>) {
    let mut args = vec![
        "-r".to_string(), fps.to_string(),
        "-i".to_string(), frame_pattern.to_string(),
    ];

    if let Some(audio) = audio_path {
        args.extend([
            "-i".to_string(), audio.to_string(),
            "-map".to_string(), "0:v".to_string(),
            "-map".to_string(), "1:a".to_string(),
            "-c:a".to_string(), "aac".to_string(),
        ]);
    } else {
        args.extend(["-map".to_string(), "0:v".to_string()]);
    }

    args.extend([
        "-pix_fmt".to_string(), "yuv420p".to_string(),
        "-preset".to_string(), "ultrafast".to_string(),
        output_path.to_string(),
        "-y".to_string(),
    ]);

    Command::new("ffmpeg")
        .args(&args)
        .status()
        .expect("Failed to run ffmpeg");
}

fn video_filter_crop(scaled_w: u32, visible_w: u32, target_h: u32) -> String {
    let crop_x = (scaled_w - visible_w) / 2;
    format!("scale=-2:{target_h},crop={visible_w}:{target_h}:{crop_x}:0")
}

fn video_filter_bars(scaled_w: u32, visible_w: u32, target_h: u32) -> String {
    if scaled_w > visible_w {
        let bar_w = (scaled_w - visible_w) / 2;
        let right_x = scaled_w - bar_w;
        let bar = |x: u32| format!("drawbox=x={x}:y=0:w={bar_w}:h={target_h}:color=black:t=fill");
        format!("scale=-2:{target_h},{},{}", bar(0), bar(right_x))
    } else {
        let pad_x = (visible_w - scaled_w) / 2;
        format!("scale=-2:{target_h},pad={visible_w}:{target_h}:{pad_x}:0:black")
    }
}

fn apply_effect_to_frame(frame_path: &PathBuf, frame_index: usize) {
    let frame_str = frame_path.to_str().unwrap();
    let mut rgb = image::open(frame_str).expect("Failed to open frame").into_rgb8();
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
