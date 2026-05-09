use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::image as vhs_image;

pub fn process(input_path: &str) -> String {
    let temp = setup();
    let frame_pattern_str = temp.frame_pattern.to_str().unwrap();

    extract_frames(input_path, frame_pattern_str);
    let has_audio = crate::audio::extract(input_path, temp.raw_wav.to_str().unwrap());

    let frames = collect_frames(&temp.dir);
    apply_effects_to_frames(&frames);

    if has_audio {
        eprintln!("Processing audio...");
        crate::audio::apply_effects(temp.raw_wav.to_str().unwrap(), temp.processed_wav.to_str().unwrap());
    }

    let fps = get_fps(input_path);
    let output_path = crate::utils::make_output_path(input_path);
    let audio_path = has_audio.then(|| temp.processed_wav.to_str().unwrap().to_string());
    reassemble(frame_pattern_str, &fps, &output_path, audio_path.as_deref());

    std::fs::remove_dir_all(&temp.dir).ok();

    output_path
}

struct TempPaths {
    dir: PathBuf,
    frame_pattern: PathBuf,
    raw_wav: PathBuf,
    processed_wav: PathBuf,
}

fn setup() -> TempPaths {
    rayon::ThreadPoolBuilder::new()
        .stack_size(8 * 1024 * 1024)
        .build_global()
        .ok();
    let dir = std::env::temp_dir().join(format!("vhsify_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("Failed to create temp dir");
    TempPaths {
        frame_pattern: dir.join("frame_%05d.jpg"),
        raw_wav: dir.join("audio_raw.wav"),
        processed_wav: dir.join("audio_vhs.wav"),
        dir,
    }
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

fn apply_effects_to_frames(frames: &[PathBuf]) {
    let total = frames.len();
    let counter = AtomicUsize::new(0);
    frames.par_iter().enumerate().for_each(|(i, frame_path)| {
        let frame_str = frame_path.to_str().unwrap();
        let mut rgb = image::open(frame_str).expect("Failed to open frame").into_rgb8();
        vhs_image::apply_effect(&mut rgb, i);
        rgb.save(frame_str).expect("Failed to save frame");
        let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
        let pct = done * 100 / total;
        if done % (total / 20).max(1) == 0 || done == total {
            let status = format!("frames {}/{} ({}%)\n", done, total, pct);
            let _ = std::fs::write("/tmp/vhsify_progress.txt", &status);
            eprint!("\r{}", status.trim());
        }
    });
    eprintln!();
    let _ = std::fs::write("/tmp/vhsify_progress.txt", "audio processing...\n");
}

fn extract_frames(input_path: &str, frame_pattern: &str) {
    Command::new("ffmpeg")
        .args(["-i", input_path, "-vf", "scale=640:480:force_original_aspect_ratio=decrease,pad=640:480:(ow-iw)/2:(oh-ih)/2", frame_pattern, "-y"])
        .status()
        .expect("Failed to run ffmpeg");
}

fn get_fps(input_path: &str) -> String {
    let output = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=r_frame_rate",
            "-of", "csv=p=0",
            input_path,
        ])
        .output()
        .expect("Failed to run ffprobe");
    String::from_utf8_lossy(&output.stdout).trim().trim_end_matches(',').to_string()
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
        output_path.to_string(),
        "-y".to_string(),
    ]);

    Command::new("ffmpeg")
        .args(&args)
        .status()
        .expect("Failed to run ffmpeg");
}
