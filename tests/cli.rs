mod common;

use tempfile::TempDir;

#[test]
fn version_includes_ntsc_rs_version() {
    let tmp = TempDir::new().unwrap();
    let output = common::vhsify(tmp.path())
        .arg("--version")
        .output()
        .unwrap();

    let ntscrs_version = env!("NTSCRS_VERSION");
    assert!(
        ntscrs_version.starts_with('v'),
        "build.rs failed to read ntsc-rs version: {ntscrs_version}"
    );
    assert!(output.status.success());
    assert_eq!(
        common::stdout(&output).trim(),
        format!(
            "vhsify v{} (ntsc-rs {})",
            env!("CARGO_PKG_VERSION"),
            ntscrs_version
        )
    );
}

#[test]
fn unsupported_extension_fails() {
    let tmp = TempDir::new().unwrap();
    let output = common::vhsify(tmp.path())
        .arg("input.txt")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(common::stderr(&output).contains("Unsupported file type: input.txt"));
}

#[test]
fn missing_extension_fails() {
    let tmp = TempDir::new().unwrap();
    let output = common::vhsify(tmp.path()).arg("input").output().unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(common::stderr(&output).contains("Unsupported file type: input"));
}

#[test]
fn missing_image_input_fails() {
    let tmp = TempDir::new().unwrap();
    let output = common::vhsify(tmp.path())
        .arg("missing.png")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(common::stderr(&output).contains("Input file not found: missing.png"));
    assert!(common::stdout(&output).is_empty());
}

#[test]
fn missing_video_input_fails() {
    let tmp = TempDir::new().unwrap();
    let output = common::vhsify(tmp.path())
        .arg("missing.mp4")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(common::stderr(&output).contains("Input file not found: missing.mp4"));
    assert!(common::stdout(&output).is_empty());
}

#[test]
fn input_that_is_a_directory_fails() {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir(tmp.path().join("input.png")).unwrap();

    let output = common::vhsify(tmp.path())
        .arg("input.png")
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(common::stderr(&output).contains("Input file not found: input.png"));
    assert!(common::stdout(&output).is_empty());
}

#[test]
fn missing_output_dir_for_image_fails() {
    let tmp = TempDir::new().unwrap();
    common::write_image(&tmp.path().join("input.png"), 64, 48);

    let output = common::vhsify(tmp.path())
        .args(["input.png", "--output-dir", "missing"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(common::stderr(&output).contains("Output directory not found: missing"));
    assert!(common::stdout(&output).is_empty());
}

#[test]
fn missing_output_dir_for_video_fails() {
    let tmp = TempDir::new().unwrap();
    // 入力の検証はファイルがあるかしか見ないので、空のファイルで足り、ffmpeg も要らない
    std::fs::write(tmp.path().join("input.mp4"), "").unwrap();

    let output = common::vhsify(tmp.path())
        .args(["input.mp4", "--output-dir", "missing"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(common::stderr(&output).contains("Output directory not found: missing"));
    assert!(common::stdout(&output).is_empty());
}

#[test]
fn output_dir_that_is_a_file_fails() {
    let tmp = TempDir::new().unwrap();
    common::write_image(&tmp.path().join("input.png"), 64, 48);
    std::fs::write(tmp.path().join("file"), "").unwrap();

    let output = common::vhsify(tmp.path())
        .args(["input.png", "--output-dir", "file"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(common::stderr(&output).contains("Output directory not found: file"));
    assert!(common::stdout(&output).is_empty());
}

#[test]
fn extension_is_case_insensitive() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.PNG");
    common::write_image(&input, 64, 48);

    let output = common::vhsify(tmp.path())
        .arg(&input)
        .arg("--output-dir")
        .arg(tmp.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", common::stderr(&output));
    assert_eq!(
        common::stdout(&output).trim(),
        format!("Saved: {}", tmp.path().join("input_vhs.PNG").display())
    );
}

#[test]
fn output_dir_option_sets_output_directory() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.png");
    let out_dir = tmp.path().join("out");
    std::fs::create_dir(&out_dir).unwrap();
    common::write_image(&input, 64, 48);

    let output = common::vhsify(tmp.path())
        .arg(&input)
        .arg("--output-dir")
        .arg(&out_dir)
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", common::stderr(&output));
    let expected = out_dir.join("input_vhs.png");
    assert!(expected.exists());
    assert_eq!(
        common::stdout(&output).trim(),
        format!("Saved: {}", expected.display())
    );
}

#[test]
fn output_name_option_sets_filename_and_keeps_extension() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.png");
    common::write_image(&input, 64, 48);

    let output = common::vhsify(tmp.path())
        .arg(&input)
        .arg("--output-dir")
        .arg(tmp.path())
        .args(["--output-name", "custom"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", common::stderr(&output));
    assert!(tmp.path().join("custom.png").exists());
}

#[test]
fn output_name_with_the_input_extension_is_not_doubled() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.png");
    common::write_image(&input, 64, 48);

    let output = common::vhsify(tmp.path())
        .arg(&input)
        .args(["--output-name", "custom.png"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", common::stderr(&output));
    assert!(tmp.path().join("custom.png").exists());
    assert!(!tmp.path().join("custom.png.png").exists());
}

#[test]
fn short_options_set_output_directory_and_filename() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.png");
    let out_dir = tmp.path().join("out");
    std::fs::create_dir(&out_dir).unwrap();
    common::write_image(&input, 64, 48);

    let output = common::vhsify(tmp.path())
        .arg(&input)
        .arg("-d")
        .arg(&out_dir)
        .args(["-n", "custom"])
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", common::stderr(&output));
    assert!(out_dir.join("custom.png").exists());
}

#[test]
fn output_defaults_to_input_directory() {
    let tmp = TempDir::new().unwrap();
    let in_dir = tmp.path().join("in");
    let cwd = tmp.path().join("cwd");
    std::fs::create_dir(&in_dir).unwrap();
    std::fs::create_dir(&cwd).unwrap();
    let input = in_dir.join("input.png");
    common::write_image(&input, 64, 48);

    let output = common::vhsify(&cwd).arg(&input).output().unwrap();

    assert!(output.status.success(), "{}", common::stderr(&output));
    let expected = in_dir.join("input_vhs.png");
    assert!(expected.exists());
    assert_eq!(
        common::stdout(&output).trim(),
        format!("Saved: {}", expected.display())
    );
}

#[test]
fn output_defaults_to_input_directory_for_bare_filename() {
    let tmp = TempDir::new().unwrap();
    common::write_image(&tmp.path().join("input.png"), 64, 48);

    let output = common::vhsify(tmp.path())
        .arg("input.png")
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", common::stderr(&output));
    assert!(tmp.path().join("input_vhs.png").exists());
    assert_eq!(common::stdout(&output).trim(), "Saved: input_vhs.png");
}
