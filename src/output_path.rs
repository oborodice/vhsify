use std::path::Path;

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
        Some(name) => format!("{}.{}", name, ext),
        None => format!(
            "{}_vhs.{}",
            path.file_stem().unwrap().to_str().unwrap(),
            ext
        ),
    }
}
