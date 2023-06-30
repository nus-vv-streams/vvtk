use std::collections::HashMap;
use std::fmt::{self, Debug};

use super::Subcommand;
use crate::pcd::{read_pcd_header, PCDHeader};
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;
use crate::ply::read_ply_header;
use clap::Parser;
use ply_rs::ply::Encoding;
use ply_rs::ply::Header as PLYHeader;
use std::path::Path;

#[derive(Parser, Debug)]
#[clap(
    about = "Get the info of a pointcloud file or directory.\nSupported formats are .pcd and .ply.\nIf no option is specified, all info will be printed."
)]
pub struct Args {
    // #[clap(short, long)]
    path: String,

    /// Get the number of points in a file
    #[clap(long, default_value_t = false)]
    num_of_points: bool,

    /// Get the format of a file
    #[clap(long, default_value_t = false)]
    format: bool,

    /// Get the number of frames in a directory
    #[clap(long, default_value_t = false)]
    num_of_frames: bool,
}

pub struct Info {
    args: Args,
}

#[derive(Clone)]
struct FileInfo {
    extension: String,
    storage_type: String,
    num_of_points: u64,
}

impl FileInfo {
    pub fn to_info_string(&self, args: &Args) -> String {
        let mut info_string: String = String::new();

        let if_print_all: bool = !(args.num_of_points || args.format);

        if if_print_all || args.format {
            info_string.push_str(&format!(
                "format: {} {}\n",
                self.extension,
                self.storage_type.to_ascii_uppercase()
            ));
        }
        if if_print_all || args.num_of_points {
            info_string.push_str(&format!("number of points: {}\n", self.num_of_points));
        }
        info_string
    }
}

impl Debug for FileInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // output format:
        // format: pcd self.storage_type
        // num_of_points: self.num_of_points
        write!(
            f,
            "format: {} {}\n",
            self.extension,
            self.storage_type.to_ascii_uppercase()
        )?;
        write!(f, "number of points: {}", self.num_of_points)
    }
}

impl From<PCDHeader> for FileInfo {
    fn from(value: PCDHeader) -> Self {
        FileInfo {
            extension: "pcd".to_string(),
            storage_type: value.data_type().to_string().to_ascii_uppercase(),
            num_of_points: value.points(),
        }
    }
}

impl From<PLYHeader> for FileInfo {
    fn from(value: PLYHeader) -> Self {
        FileInfo {
            extension: "ply".to_string(),
            storage_type: match value.encoding {
                Encoding::Ascii => "ASCII".to_string(),
                _ => "BINARY".to_string(),
            },
            num_of_points: value.elements.get("vertex").unwrap().count as u64,
        }
    }
}

struct DirInfo {
    extension: String,
    storage_type: String,
    num_of_frames: u64,
    avg_num_of_points: f64,
}

impl DirInfo {
    pub fn to_info_string(&self, args: &Args) -> String {
        let mut info_string: String = String::new();

        let if_print_all: bool = !(args.num_of_points || args.format);

        if if_print_all || args.format {
            info_string.push_str(&format!(
                "format: {} {}\n",
                self.extension,
                self.storage_type.to_ascii_uppercase()
            ));
        }

        if if_print_all || args.num_of_frames {
            info_string.push_str(&format!("number of frames: {}\n", self.num_of_frames));
        }

        if if_print_all || args.num_of_points {
            info_string.push_str(&format!(
                "average number of points: {:.2}\n",
                self.avg_num_of_points
            ));
        }
        info_string
    }
}
impl Debug for DirInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "format: {} {}\n",
            self.extension,
            self.storage_type.to_ascii_uppercase()
        )?;
        write!(f, "number of frames: {}\n", self.num_of_frames)?;
        write!(f, "average number of points: {:.2}", self.avg_num_of_points)
    }
}

impl Info {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        Box::from(Info {
            args: Args::parse_from(args),
        })
    }

    fn handle_file(&self, path: &Path) -> Result<FileInfo, String> {
        if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
            let file_info: Option<FileInfo> = match ext {
                "ply" => Some(read_ply_header(path).unwrap().into()),
                "pcd" => Some(read_pcd_header(path).unwrap().into()),
                _ => None,
            };
            return file_info.ok_or(format!("Unsupported file format: {}", ext));
        }
        Err("Unsupported file format.".to_string())
    }

    fn handle_dir(&self, path: &Path) {
        let mut dir_infos: HashMap<String, DirInfo> = HashMap::new();
        for file_entry in path.read_dir().unwrap() {
            let file_entry = file_entry.unwrap();
            let file_path = file_entry.path();
            if file_path.is_file() {
                // if let Some(file_info) = self.handle_file(&file_path)
                if let Ok(file_info) = self.handle_file(&file_path) {
                    let ext = file_info.extension.clone();
                    let storage_type = file_info.storage_type.clone();
                    let format_key = format!("{}_{}", ext, storage_type);
                    let dir_info = dir_infos.entry(format_key).or_insert(DirInfo {
                        extension: ext,
                        storage_type: storage_type,
                        num_of_frames: 0,
                        avg_num_of_points: 0f64,
                    });
                    dir_info.num_of_frames += 1;
                    dir_info.avg_num_of_points = (dir_info.avg_num_of_points
                        * (dir_info.num_of_frames - 1) as f64)
                        / dir_info.num_of_frames as f64
                        + file_info.num_of_points as f64 / dir_info.num_of_frames as f64;
                }
            }
        }

        if dir_infos.is_empty() {
            println!("No files found in directory, supported formats are: pcd, ply");
            return;
        }

        for (_key, value) in dir_infos {
            println!("{}", value.to_info_string(&self.args));
        }
    }
}

impl Subcommand for Info {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        if messages.is_empty() {
            println!("self.args {:?}", self.args);
            let path = Path::new(&self.args.path);

            if path.is_file() {
                let file_info = self.handle_file(&path);
                match file_info {
                    Ok(file_info) => println!("{}", file_info.to_info_string(&self.args)),
                    Err(err_msg) => println!("{}", err_msg),
                }
            } else if path.is_dir() {
                self.handle_dir(&path);
            } else {
                println!("Path is neither a file nor a directory");
            }

            channel.send(PipelineMessage::End);
        } else {
            for message in messages {
                channel.send(message);
            }
        }
    }
}
