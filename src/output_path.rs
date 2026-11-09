use std::path::{Path, PathBuf};

pub(crate) fn build(
    input_path: &str,
    output_dir: Option<&str>,
    output_name: Option<&str>,
) -> String {
    let filename = filename(input_path, output_name);
    let dir = match output_dir {
        Some(dir) => Path::new(dir),
        None => Path::new(input_path).parent().unwrap_or(Path::new("")),
    };
    dir.join(filename).to_string_lossy().into_owned()
}

fn filename(input_path: &str, output_name: Option<&str>) -> String {
    let path = Path::new(input_path);
    let ext = path.extension().unwrap().to_str().unwrap();
    match output_name {
        // 入力と同じ拡張子がすでに付いていれば、重ねて付けない
        Some(name)
            if Path::new(name)
                .extension()
                .is_some_and(|name_ext| name_ext.eq_ignore_ascii_case(ext)) =>
        {
            name.to_string()
        }
        Some(name) => format!("{}.{}", name, ext),
        None => format!(
            "{}_vhs.{}",
            path.file_stem().unwrap().to_str().unwrap(),
            ext
        ),
    }
}

/// 書き出しの途中で中断・失敗しても、壊れたファイルが残ったり既存のファイルが失われたりしないよう、
/// `write` に別名のパスを渡して書き出させ、成功したら出力先へ名前を変える。失敗したら別名のファイルを消す。
/// `error` は名前の変更に失敗したときのメッセージ
pub(crate) fn write_via_partial(
    output_path: &str,
    error: String,
    write: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<(), String> {
    let partial = partial_path(output_path);
    let result =
        write(&partial).and_then(|()| std::fs::rename(&partial, output_path).map_err(|_| error));
    if result.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    result
}

/// 出力先と同じフォルダの隠しファイル。名前の変更が一瞬で終わるよう同じフォルダにし、
/// ffmpeg や image クレートが拡張子で形式を決めるので拡張子は残す
fn partial_path(output_path: &str) -> PathBuf {
    let path = Path::new(output_path);
    let stem = path.file_stem().unwrap().to_string_lossy();
    let ext = path.extension().unwrap().to_string_lossy();
    path.with_file_name(format!(".{stem}.partial.{ext}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(dir: &str, file: &str) -> String {
        Path::new(dir).join(file).to_string_lossy().into_owned()
    }

    #[test]
    fn default_output_goes_next_to_the_input_with_a_vhs_suffix() {
        assert_eq!(
            build("in/photo.jpg", None, None),
            path("in", "photo_vhs.jpg")
        );
    }

    #[test]
    fn output_dir_replaces_the_input_directory() {
        assert_eq!(
            build("in/photo.jpg", Some("out"), None),
            path("out", "photo_vhs.jpg")
        );
    }

    #[test]
    fn output_name_gets_the_input_extension() {
        assert_eq!(
            build("in/photo.jpg", None, Some("custom")),
            path("in", "custom.jpg")
        );
    }

    #[test]
    fn output_name_with_the_input_extension_is_used_as_is() {
        assert_eq!(
            build("in/photo.jpg", None, Some("custom.jpg")),
            path("in", "custom.jpg")
        );
        assert_eq!(
            build("in/photo.JPG", None, Some("custom.jpg")),
            path("in", "custom.jpg")
        );
    }

    #[test]
    fn output_name_with_another_extension_still_gets_the_input_extension() {
        // 出力の形式は常に入力に合わせるので、ほかのドットは名前の一部として扱う
        assert_eq!(
            build("in/photo.jpg", None, Some("my.photo")),
            path("in", "my.photo.jpg")
        );
        assert_eq!(
            build("in/photo.jpg", None, Some("custom.png")),
            path("in", "custom.png.jpg")
        );
    }
}
