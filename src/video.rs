use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::image as vhs_image;

pub fn process(input_path: &str) -> String {
    let temp_dir = std::env::temp_dir().join(format!("vhsify_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    let frame_pattern = temp_dir.join("frame_%05d.jpg");
    let frame_pattern_str = frame_pattern.to_str().unwrap();
    let raw_wav = temp_dir.join("audio_raw.wav");
    let processed_wav = temp_dir.join("audio_vhs.wav");

    extract_frames(input_path, frame_pattern_str);
    let has_audio = crate::audio::extract(input_path, raw_wav.to_str().unwrap());

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

    if has_audio {
        eprintln!("Processing audio...");
        crate::audio::apply_effects(raw_wav.to_str().unwrap(), processed_wav.to_str().unwrap());
    }

    let fps = get_fps(input_path);
    let output_path = crate::make_output_path(input_path);
    let audio_path = if has_audio {
        Some(processed_wav.to_str().unwrap().to_string())
    } else {
        None
    };
    reassemble(frame_pattern_str, &fps, &output_path, audio_path.as_deref());

    std::fs::remove_dir_all(&temp_dir).ok();

    output_path
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
