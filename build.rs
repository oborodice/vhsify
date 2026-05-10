use std::fs;

fn main() {
    let cargo_toml = fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");
    let ntscrs_version = cargo_toml
        .lines()
        .find(|l| l.contains("ntsc-rs") && l.contains("tag"))
        .and_then(|l| l.split("tag = \"").nth(1))
        .and_then(|l| l.split('"').next())
        .unwrap_or("unknown");
    println!("cargo:rustc-env=NTSCRS_VERSION={}", ntscrs_version);
}
