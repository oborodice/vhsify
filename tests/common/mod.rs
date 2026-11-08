use std::path::Path;
use std::process::{Command, Output};

pub fn vhsify(cwd: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vhsify"));
    command.current_dir(cwd);
    command
}

/// Writes a white image whose format is chosen by the file extension
pub fn write_image(path: &Path, width: u32, height: u32) {
    image::RgbImage::from_pixel(width, height, image::Rgb([255, 255, 255]))
        .save(path)
        .expect("Failed to write image");
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
