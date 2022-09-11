use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use clap::Parser;
use ply_rs::ply::{self, Property};

use super::Subcommand;
use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::formats::PointCloud;
use crate::pcd::read_pcd_file;
use crate::pipeline::PipelineMessage;

#[derive(Parser)]
struct Args {
    /// Files, glob patterns, directories
    files: Vec<OsString>,
}

pub struct Read {
    args: Args,
}

impl Read {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        Box::from(Read {
            args: Args::parse_from(args),
        })
    }
}

impl Subcommand for Read {
    fn handle(&mut self, message: PipelineMessage, out: &Sender<PipelineMessage>) {
        if let PipelineMessage::End = message {
            let mut files = find_all_files(&self.args.files);
            files.sort();
            for file in files {
                if let Some(ext) = file.extension().and_then(|ext| ext.to_str()) {
                    let point_cloud = match ext {
                        "ply" => read_ply(file),
                        "pcd" => read_pcd_file(file).map(PointCloud::from).ok(),
                        _ => continue,
                    };

                    if let Some(pc) = point_cloud {
                        out.send(PipelineMessage::PointCloud(pc));
                    }
                }
            }
        } else {
            out.send(message);
        }
    }
}

fn find_all_files(os_strings: &Vec<OsString>) -> Vec<PathBuf> {
    let mut files_to_convert = vec![];
    for file_str in os_strings {
        let path = Path::new(&file_str);
        if path.is_dir() {
            files_to_convert.extend(expand_directory(path));
        } else {
            files_to_convert.push(path.to_path_buf());
        }
    }
    files_to_convert
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
        ply_files.push(entry);
    }

    ply_files
}

fn read_ply(path_buf: PathBuf) -> Option<PointCloud<PointXyzRgba>> {
    let vertex_parser = ply_rs::parser::Parser::<PointXyzRgba>::new();
    let f = std::fs::File::open(path_buf.clone())
        .expect(&format!("Unable to open file {:?}", &path_buf));
    let mut f = std::io::BufReader::new(f);

    let header = {
        match vertex_parser.read_header(&mut f) {
            Ok(h) => h,
            Err(e) => {
                println!("Failed to convert {:?}\n{e}", &path_buf);
                return None;
            }
        }
    };

    let mut vertex_list = Vec::new();
    for (_, element) in &header.elements {
        if element.name.as_str() == "vertex" {
            vertex_list = match vertex_parser.read_payload_for_element(&mut f, element, &header) {
                Ok(v) => v,
                Err(e) => {
                    println!("Failed to convert {:?}\n{e}", &path_buf);
                    return None;
                }
            }
        }
    }

    Some(PointCloud {
        number_of_points: vertex_list.len(),
        points: vertex_list,
    })
}

impl ply::PropertyAccess for PointXyzRgba {
    fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    fn set_property(&mut self, key: &String, property: Property) {
        match (key.as_ref(), property) {
            ("x", ply::Property::Float(v)) => self.x = v,
            ("y", ply::Property::Float(v)) => self.y = v,
            ("z", ply::Property::Float(v)) => self.z = v,
            ("red", ply::Property::UChar(v)) => self.r = v,
            ("green", ply::Property::UChar(v)) => self.g = v,
            ("blue", ply::Property::UChar(v)) => self.b = v,
            ("alpha", ply::Property::UChar(v)) => self.a = v,
            _ => {}
        }
    }
}
