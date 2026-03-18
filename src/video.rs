use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::image as vhs_image;

pub fn process(input_path: &str) -> String {
    let temp_dir = std::env::temp_dir().join(format!("vhsify_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    let frame_pattern = temp_dir.join("frame_%05d.jpg");
    let frame_pattern_str = frame_pattern.to_str().unwrap();

    extract_frames(input_path, frame_pattern_str);

    let mut frames: Vec<_> = std::fs::read_dir(&temp_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "jpg").unwrap_or(false))
        .collect();
    frames.sort();

    let total = frames.len();
    let counter = AtomicUsize::new(0);
    frames.par_iter().enumerate().for_each(|(i, frame_path)| {
        let frame_str = frame_path.to_str().unwrap();
        let mut rgb = image::open(frame_str).expect("Failed to open frame").into_rgb8();
        vhs_image::apply_effect(&mut rgb, i);
        rgb.save(frame_str).expect("Failed to save frame");
        let done = counter.fetch_add(1, Ordering::Relaxed) + 1;
        eprint!("\rProcessing frame {}/{}", done, total);
    });
    eprintln!();

    let fps = get_fps(input_path);
    let output_path = crate::make_output_path(input_path);
    reassemble(input_path, frame_pattern_str, &fps, &output_path);

    std::fs::remove_dir_all(&temp_dir).ok();

    output_path
}

fn extract_frames(input_path: &str, frame_pattern: &str) {
    Command::new("ffmpeg")
        .args(["-i", input_path, "-vf", "scale=-2:480", frame_pattern, "-y"])
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
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn reassemble(input_path: &str, frame_pattern: &str, fps: &str, output_path: &str) {
    Command::new("ffmpeg")
        .args([
            "-r", fps,
            "-i", frame_pattern,
            "-i", input_path,
            "-map", "0:v",
            "-map", "1:a?",
            "-c:a", "copy",
            "-pix_fmt", "yuv420p",
            output_path,
            "-y",
        ])
        .status()
        .expect("Failed to run ffmpeg");
}

