mod audio;
mod image;
mod interrupt;
mod layout;
mod orientation;
mod output_path;
mod video;

use std::path::Path;

use clap::Parser;

#[derive(Parser)]
#[command(version = concat!("v", env!("CARGO_PKG_VERSION"), " (ntsc-rs ", env!("NTSCRS_VERSION"), ")"))]
struct Args {
    #[arg(help = "Input file (JPEG, PNG, WebP, AVIF, MP4, MOV, AVI, MKV)")]
    input: String,
    #[arg(short, long, value_enum, default_value_t = layout::ScaleMode::Bars, help = "How to handle wide content: fit into 4:3 with black bars, or crop sides to 4:3")]
    mode: layout::ScaleMode,
    #[arg(short = 'd', long, help = "Output directory (default: same as input)")]
    output_dir: Option<String>,
    #[arg(
        short = 'n',
        long,
        help = "Output filename; the input's extension is added unless the name already has it (default: <input>_vhs)"
    )]
    output_name: Option<String>,
}

fn main() {
    let args = Args::parse();
    match run(args) {
        Ok(output_path) => println!("Saved: {}", output_path),
        // Ctrl+C は ffmpeg にも届き、ffmpeg の失敗として返ってくることがあるため、どの段階のエラーでも中断として報告する
        Err(_) if interrupt::requested() => {
            eprintln!("{}", interrupt::MESSAGE);
            std::process::exit(interrupt::EXIT_CODE);
        }
        Err(message) => {
            eprintln!("{}", message);
            std::process::exit(1);
        }
    }
}

fn run(args: Args) -> Result<String, String> {
    let input_path = &args.input;
    let output_dir = args.output_dir.as_deref();
    let output_name = args.output_name.as_deref();

    match validate(input_path, output_dir)? {
        Media::Image => Ok(image::process(
            input_path,
            args.mode,
            output_dir,
            output_name,
        )),
        Media::Video => video::process(input_path, args.mode, output_dir, output_name),
    }
}

enum Media {
    Image,
    Video,
}

fn validate(input_path: &str, output_dir: Option<&str>) -> Result<Media, String> {
    let media =
        media_type(input_path).ok_or_else(|| format!("Unsupported file type: {}", input_path))?;
    if !Path::new(input_path).is_file() {
        return Err(format!("Input file not found: {}", input_path));
    }
    if let Some(dir) = output_dir.filter(|dir| !Path::new(dir).is_dir()) {
        return Err(format!("Output directory not found: {}", dir));
    }
    Ok(media)
}

fn media_type(path: &str) -> Option<Media> {
    match file_extension(path).as_str() {
        "jpg" | "jpeg" | "png" | "webp" | "avif" => Some(Media::Image),
        "mp4" | "mov" | "avi" | "mkv" => Some(Media::Video),
        _ => None,
    }
}

fn file_extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}
