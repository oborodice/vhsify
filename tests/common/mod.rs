use std::path::Path;
use std::process::{Command, Output};

pub fn vhsify(cwd: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vhsify"));
    command.current_dir(cwd);
    command
}

pub fn write_png(path: &Path, width: u32, height: u32) {
    image::RgbImage::new(width, height)
        .save(path)
        .expect("Failed to write PNG");
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
