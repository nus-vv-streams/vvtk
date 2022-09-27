use anyhow::{bail, Result};
use clap::Parser;
use rayon::prelude::*;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use vivotk::pcd::{write_pcd_file, PCDDataType, PointCloudData};
use vivotk::transform::ply_to_pcd;

/// Converts ply files that are in Point_XYZRGBA format to pcd files
///
/// This assumes that the given ply files contain vertices and that the vertices
/// are the first field in the ply file.
#[derive(Parser)]
struct Args {
    #[clap(short, long)]
    output_dir: String,

    /// Storage type can be either "ascii" or "binary"
    #[clap(short, long, default_value = "binary")]
    storage_type: PCDDataType,

    /// Files, glob patterns, directories
    files: Vec<OsString>,
}

fn main() {
    let args: Args = Args::parse();

    let files_to_convert = filter_for_ply_files(args.files);
    let output_path = Path::new(&args.output_dir);
    std::fs::create_dir_all(output_path).expect("Failed to create output directory");
    println!("Converting {} files...", files_to_convert.len());
    files_to_convert
        .par_iter()
        .map(|f| (f.as_path(), ply_to_pcd(f.as_path()).unwrap_or(None)))
        .for_each(|(f, data)| {
            if let Some(pcd) = data {
                _ = materialize_pcd(f, &output_path, args.storage_type, &pcd);
            }
        })
}

fn materialize_pcd(
    file_path: &Path,
    output_path: &Path,
    storage_type: PCDDataType,
    pcd: &PointCloudData,
) -> Result<()> {
    let filename = Path::new(file_path.file_name().unwrap()).with_extension("pcd");
    let output_file = output_path.join(filename);
    if let Err(e) = write_pcd_file(&pcd, storage_type, &output_file) {
        bail!(
            "Failed to write {:?} to {:?}\n{e}",
            file_path.as_os_str(),
            output_file.into_os_string()
        );
    }
    Ok(())
}

fn filter_for_ply_files(os_strings: Vec<OsString>) -> Vec<PathBuf> {
    let mut files_to_convert = vec![];
    for file_str in os_strings {
        let path = Path::new(&file_str);
        if path.is_dir() {
            files_to_convert.extend(expand_directory(path));
        } else if is_ply_file(path) {
            files_to_convert.push(path.to_path_buf());
        }
    }
    files_to_convert
}

fn is_ply_file(p: &Path) -> bool {
    p.extension().map(|f| "ply".eq(f)).unwrap_or(false)
}

fn expand_directory(p: &Path) -> Vec<PathBuf> {
    let mut ply_files = vec![];
    let dir_entry = p.read_dir().unwrap();
    for entry in dir_entry {
        let entry = entry.unwrap().path();
        if !entry.is_file() {
            // We do not recursively search
            continue;
        }

        if is_ply_file(&entry) {
            ply_files.push(entry);
        }
    }

    ply_files
}
