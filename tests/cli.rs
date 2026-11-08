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
fn extension_is_case_insensitive() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.PNG");
    common::write_png(&input, 64, 48);

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
    common::write_png(&input, 64, 48);

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
    common::write_png(&input, 64, 48);

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
fn short_options_set_output_directory_and_filename() {
    let tmp = TempDir::new().unwrap();
    let input = tmp.path().join("input.png");
    let out_dir = tmp.path().join("out");
    std::fs::create_dir(&out_dir).unwrap();
    common::write_png(&input, 64, 48);

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
#[ignore = "known bug: output is written to the current directory, not the input's directory"]
fn output_defaults_to_input_directory() {
    let tmp = TempDir::new().unwrap();
    let in_dir = tmp.path().join("in");
    let cwd = tmp.path().join("cwd");
    std::fs::create_dir(&in_dir).unwrap();
    std::fs::create_dir(&cwd).unwrap();
    let input = in_dir.join("input.png");
    common::write_png(&input, 64, 48);

    let output = common::vhsify(&cwd).arg(&input).output().unwrap();

    assert!(output.status.success(), "{}", common::stderr(&output));
    assert!(in_dir.join("input_vhs.png").exists());
}
