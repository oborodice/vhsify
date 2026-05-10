mod audio;
mod exif;
mod image;
mod utils;
mod video;

use clap::Parser;

#[derive(Parser)]
#[command(version = concat!("v", env!("CARGO_PKG_VERSION"), " (ntsc-rs ", env!("NTSCRS_VERSION"), ")"))]
struct Args {
    #[arg(help = "Input file (JPEG, PNG, WebP, AVIF, MP4, MOV, AVI, MKV)")]
    input: String,
    #[arg(short, long, value_enum, default_value_t = utils::ScaleMode::Bars, help = "How to handle wide content: fit into 4:3 with black bars, or crop sides to 4:3")]
    mode: utils::ScaleMode,
    #[arg(short = 'd', long, help = "Output directory (default: same as input)")]
    output_dir: Option<String>,
    #[arg(short = 'n', long, help = "Output filename without extension (default: <input>_vhs)")]
    output_name: Option<String>,
}

fn main() {
    let args = Args::parse();
    let input_path = &args.input;
    let output_dir = args.output_dir.as_deref();
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
