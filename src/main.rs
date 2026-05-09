mod audio;
mod exif;
mod image;
mod utils;
mod video;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: vhsify <INPUT>");
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = if is_video(input_path) {
        video::process(input_path)
    } else if is_image(input_path) {
        image::process(input_path)
    } else {
        eprintln!("Unsupported file type: {}", input_path);
        std::process::exit(1);
    };
    println!("Saved: {}", output_path);
}

fn is_video(path: &str) -> bool {
    matches!(
        ext(path),
        "mp4" | "mov" | "avi" | "mkv"
    )
}

fn is_image(path: &str) -> bool {
    matches!(
        ext(path),
        "jpg" | "jpeg" | "png" | "webp" | "avif"
    )
}

fn ext(path: &str) -> &str {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
}
