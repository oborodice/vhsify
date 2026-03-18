use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: vhsify <INPUT>");
        std::process::exit(1);
    }

    let input_path = &args[1];
    let img = image::open(input_path).expect("Failed to open image");
    println!("Loaded: {} ({}x{})", input_path, img.width(), img.height());
}
