use clap::Parser;
use kdam::tqdm;
use ply_rs::{ply, ply::Property};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use vivotk::pcd::{
    write_pcd_file, PCDDataType, PCDField, PCDFieldSize, PCDFieldType, PCDHeader, PCDVersion,
    PointCloudData,
};

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
    let mut count = 0;

    let vertex_parser = ply_rs::parser::Parser::<Vertex>::new();
    'outer: for file_path in tqdm!(files_to_convert.into_iter()) {
        
        let f = std::fs::File::open(file_path.clone()).unwrap();
        let mut f = std::io::BufReader::new(f);

        let header = {
            match vertex_parser.read_header(&mut f) {
                Ok(h) => h,
                Err(e) => {
                    println!("Failed to convert {:?}\n{e}", file_path.into_os_string());
                    continue;
                }
            }
        };

        let mut vertex_list = Vec::new();
        for (_, element) in &header.elements {
            // we could also just parse them in sequence, but the file format might change
            if element.name.as_str() == "vertex" {
                vertex_list = match vertex_parser.read_payload_for_element(&mut f, element, &header)
                {
                    Ok(v) => v,
                    Err(e) => {
                        println!("Failed to convert {:?}\n{e}", file_path.into_os_string());
                        continue 'outer;
                    }
                }
            }
        }
        if vertex_list.is_empty() {
            println!(
                "{:?} does not contain any vertices..skipping this file",
                file_path.into_os_string()
            );
            continue;
        }

        let pcd = create_pcd(vertex_list);

        let filename = Path::new(file_path.file_name().unwrap()).with_extension("pcd");
        let output_file = output_path.join(filename);
        if let Err(e) = write_pcd_file(&pcd, args.storage_type, &output_file) {
            println!(
                "Failed to write {:?} to {:?}\n{e}",
                file_path.into_os_string(),
                output_file.into_os_string()
            );
            continue;
        }
        count += 1;
    }

    println!("Successfully converted {count} files");
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

fn create_pcd(vertices: Vec<Vertex>) -> PointCloudData {
    let header = PCDHeader::new(
        PCDVersion::V0_7,
        vec![
            PCDField::new("x".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new("y".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new("z".to_string(), PCDFieldSize::Four, PCDFieldType::Float, 1).unwrap(),
            PCDField::new(
                "rgb".to_string(),
                PCDFieldSize::Four,
                PCDFieldType::Float,
                1,
            )
            .unwrap(),
        ],
        vertices.len() as u64,
        1,
        [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        vertices.len() as u64,
    )
    .unwrap();

    let bytes: &[u8] = bytemuck::cast_slice(&vertices[..]);
    PointCloudData::new(header, bytes.to_vec()).unwrap()
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    x: f32,
    y: f32,
    z: f32,
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl ply::PropertyAccess for Vertex {
    fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            red: 0,
            green: 0,
            blue: 0,
            alpha: 255,
        }
    }

    fn set_property(&mut self, key: &String, property: Property) {
        match (key.as_ref(), property) {
            ("x", ply::Property::Float(v)) => self.x = v,
            ("y", ply::Property::Float(v)) => self.y = v,
            ("z", ply::Property::Float(v)) => self.z = v,
            ("red", ply::Property::UChar(v)) => self.red = v,
            ("green", ply::Property::UChar(v)) => self.green = v,
            ("blue", ply::Property::UChar(v)) => self.blue = v,
            ("alpha", ply::Property::UChar(v)) => self.alpha = v,
            _ => {}
        }
    }
}
