use std::path::Path;

pub(crate) fn make_output_path(
    input_path: &str,
    output_dir: Option<&str>,
    output_name: Option<&str>,
) -> String {
    let filename = make_output_filename(input_path, output_name);
    let dir = match output_dir {
        Some(dir) => Path::new(dir),
        None => Path::new(input_path).parent().unwrap_or(Path::new("")),
    };
    dir.join(filename).to_string_lossy().into_owned()
}

fn make_output_filename(input_path: &str, output_name: Option<&str>) -> String {
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
