mod audio;
mod exif;
mod image;
mod utils;
mod video;

use clap::Parser;

#[derive(Parser)]
struct Args {
    input: String,
    #[arg(long, value_enum, default_value_t = utils::ScaleMode::Bars)]
    mode: utils::ScaleMode,
    #[arg(long)]
    output: Option<String>,
    #[arg(long)]
    output_name: Option<String>,
}

fn main() {
    let args = Args::parse();
    let input_path = &args.input;
    let output_dir = args.output.as_deref();
    let output_name = args.output_name.as_deref();

    let output_path = if is_image(input_path) {
        image::process(input_path, args.mode, output_dir, output_name)
    } else if is_video(input_path) {
        video::process(input_path, args.mode, output_dir, output_name)
    } else {
        eprintln!("Unsupported file type: {}", input_path);
        std::process::exit(1);
    };
    println!("Saved: {}", output_path);
}

fn is_image(path: &str) -> bool {
    matches!(
        file_extension(path).as_str(),
        "jpg" | "jpeg" | "png" | "webp" | "avif"
    )
}

fn is_video(path: &str) -> bool {
    matches!(
        file_extension(path).as_str(),
        "mp4" | "mov" | "avi" | "mkv"
    )
}

fn file_extension(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}
