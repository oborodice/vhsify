mod exif;
mod image;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: vhsify <INPUT>");
        std::process::exit(1);
    }

    let input_path = &args[1];
    let output_path = image::process(input_path);
    println!("Saved: {}", output_path);
}
