#[derive(Clone, clap::ValueEnum)]
pub(crate) enum ScaleMode {
    Bars,
    Crop,
}

pub(crate) fn make_output_path(input_path: &str, output_dir: Option<&str>, output_name: Option<&str>) -> String {
    let filename = make_output_filename(input_path, output_name);
    match output_dir {
        Some(dir) => format!("{}/{}", dir, filename),
        None => filename,
    }
}

fn make_output_filename(input_path: &str, output_name: Option<&str>) -> String {
    let path = std::path::Path::new(input_path);
    let ext = path.extension().unwrap().to_str().unwrap();
    match output_name {
        Some(name) => format!("{}.{}", name, ext),
        None => format!("{}_vhs.{}", path.file_stem().unwrap().to_str().unwrap(), ext),
    }
}
