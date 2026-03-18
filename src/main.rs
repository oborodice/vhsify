mod audio;
mod exif;
mod image;
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
    } else {
        image::process(input_path)
    };
    println!("Saved: {}", output_path);
}

fn is_video(path: &str) -> bool {
    matches!(
        std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or(""),
        "mp4" | "mov" | "avi" | "mkv"
    )
}

pub(crate) fn make_output_path(input_path: &str) -> String {
    let path = std::path::Path::new(input_path);
    let stem = path.file_stem().unwrap().to_str().unwrap();
    let ext = path.extension().unwrap().to_str().unwrap();
    format!("{}_vhs.{}", stem, ext)
}
