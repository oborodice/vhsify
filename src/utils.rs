#[derive(Clone, clap::ValueEnum)]
pub(crate) enum ScaleMode {
    Bars,
    Crop,
}

pub(crate) fn make_output_path(input_path: &str, output_dir: Option<&str>) -> String {
    let path = std::path::Path::new(input_path);
    let stem = path.file_stem().unwrap().to_str().unwrap();
    let ext = path.extension().unwrap().to_str().unwrap();
    let filename = format!("{}_vhs.{}", stem, ext);
    match output_dir {
        Some(dir) => format!("{}/{}", dir, filename),
        None => filename,
    }
}
