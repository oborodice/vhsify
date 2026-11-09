// テストのファイルごとにこのモジュールを別々にコンパイルし、それぞれ一部しか使わない
#![allow(dead_code)]

pub mod luma;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub fn vhsify(cwd: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vhsify"));
    command.current_dir(cwd);
    command
}

/// 白い画像を書き出す。形式はファイルの拡張子で決まる
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

/// `dir` の中身の名前を、隠しファイルも含めて並べ替えて返す
pub fn dir_entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[track_caller]
pub fn run(cwd: &Path, input: &Path, args: &[&str]) -> PathBuf {
    let output = vhsify(cwd).arg(input).args(args).output().unwrap();
    assert!(output.status.success(), "{}", stderr(&output));

    let stdout = stdout(&output);
    let path = stdout
        .trim()
        .strip_prefix("Saved: ")
        .unwrap_or_else(|| panic!("unexpected stdout: {stdout}"));
    PathBuf::from(path)
}
