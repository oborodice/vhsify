pub(crate) fn make_output_path(input_path: &str) -> String {
    let path = std::path::Path::new(input_path);
    let stem = path.file_stem().unwrap().to_str().unwrap();
    let ext = path.extension().unwrap().to_str().unwrap();
    format!("{}_vhs.{}", stem, ext)
}
