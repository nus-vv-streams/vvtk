use std::str::FromStr;
use kdam::tqdm;
use ply_rs::{ply, ply::Property, parser, writer, ply::Encoding};
use byteorder::{NativeEndian, ByteOrder, LittleEndian, BigEndian};
// use std::ffi::OsString;
use std::{path::{Path, PathBuf}, ffi::OsString, io::BufWriter};
// use vivotk::pcd::{
//     write_pcd_file, PCDDataType, PCDField, PCDFieldSize, PCDFieldType, PCDHeader, PCDVersion,
//     PointCloudData,
// };
use vivotk::pcd::{PCDDataType};
use clap::{command, Command, arg, ArgAction, Arg, Parser};

mod ply_to_pcd_mod;
use ply_to_pcd_mod::ply_to_pcd;

mod ply_play_mod;

use std::fs::File;
use vivotk::pcd::{read_pcd_file, write_pcd_file};


#[derive(Parser, Debug)]
enum VVSubCommand {
    #[clap(name = "convert")]
    Convert(ConvertArgs),
    #[clap(name = "play")]
    Play(ply_play_mod::Args),
}

#[derive(Parser, Debug)]
struct ConvertArgs {
    #[clap(short, long)]
    output: String,

    #[clap(long, default_value = "pcd")]
    output_format: ConvertOutputFormat, 

    #[clap(short, long, default_value = "binary")]
    storage_type: PCDDataType,

    #[clap(short, long)]
    input: Vec<OsString>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum ConvertOutputFormat {
    PLY,
    PCD,
    PNG,
    MP4,
}

impl ToString for ConvertOutputFormat {
    fn to_string(&self) -> String {
        match self {
            ConvertOutputFormat::PLY => "ply",
            ConvertOutputFormat::PCD => "pcd",
            ConvertOutputFormat::PNG => "png",
            ConvertOutputFormat::MP4 => "mp4",
        }
        .to_string()
    }
}

impl FromStr for ConvertOutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ply" => Ok(ConvertOutputFormat::PLY),
            "pcd" => Ok(ConvertOutputFormat::PCD),
            "png" => Ok(ConvertOutputFormat::PNG),
            "mp4" => Ok(ConvertOutputFormat::MP4),
            _ => Err(format!("{} is not a valid output format", s)),
        }
    }
}

fn main() {
    let subcommand: VVSubCommand = VVSubCommand::parse();

    match subcommand {
        VVSubCommand::Convert(args) => {
            // Handle the "convert" subcommand
            println!("convert mode");
            println!("rest of args: {:?} ", args);
        }
        VVSubCommand::Play(args) => {
            // Handle the "play" subcommand
            println!("play mode");
            println!("rest of args: {:?} ", args);
        }
    }


    // let matches = command!() // requires `cargo` feature
    //     .subcommand(
    //         Command::new("convert")
    //             .about("does convert things")
    //             .arg(
    //                 arg!(--input  <INPUT_DIR>  "specify input directory")
    //                 .required(true).action(clap::ArgAction::Append)
                    
    //             )
    //             .arg(
    //                 arg!(--output <OUTPUT_DIR> "specify output directory")
    //                 .required(true)
    //             )
    //             .arg(
    //                 arg!(--out_format <OUTPUT_FORMAT> "specify output format, can be ply or pcd")
    //                 .default_value("pcd")
    //                 .required(false)
    //             )
    //             .arg(
    //                 arg!(--storage_type <STORAGE_TYPE> "specify output type, can be ascii or binary")
    //                 .default_value("binary")
    //                 .required(false)
    //             )
    //             , 
    //     )
    //     .subcommand(
    //         Command::new("play")
    //             .about("does play things")
    //             // to allow any args, will be parsed later

    //     )
    //     .get_matches();

    //     match matches.subcommand() {
    //         Some(("convert", sub_matches)) => {
    //             // println!("subcommand matches: {:?}", sub_matches);
    //             let input_dirs: Vec<OsString> = sub_matches.get_many::<String>("input")
    //                                                     .unwrap_or_default().map(|x| x.clone().into()).collect();

    //             println!("input_dir: {:?}", &input_dirs);
    //             let output_dir = sub_matches.get_one::<String>("output").unwrap();
    //             println!("output_dir: {:?}", output_dir);
    //             let out_format = sub_matches.get_one::<String>("out_format").unwrap();
    //             println!("out_format: {:?}", out_format);
    //             let storage_type = sub_matches.get_one::<String>("storage_type").unwrap();
                
    //             // convert to type Vec<OsString>
    //             let storage_pcd_data_type: PCDDataType = storage_type.parse().unwrap();
    //             println!("storage_type_PCDDataType: {:?}", storage_pcd_data_type); 
                 
    //             // ply_to_pcd(output_dir.clone(), storage_pcd_data_type, input_dirs.clone());

    //             // pcd_to_pcd(output_dir.clone(), storage_pcd_data_type, input_dirs.clone());           

    //             ply_to_ply(output_dir.clone(), storage_pcd_data_type, input_dirs.clone());   
    //         }
    //         Some(("play", sub_matches)) => {
    //             println!("play");
    //             println!("subcommand matches: {:?}", sub_matches);
    //             // let args = ply_play_mod::Args::parse_from();
    //             // ply_play::Args::parse_from(&["play", "test", "test2"]);
    //         }
    //         _ => unreachable!(),
    //     }

    // println!("Hello, world end!");
}

fn ply_to_ply(output_dir:String, storage_type:PCDDataType, files:Vec<OsString>){
    let files_to_convert = filter_files_with_extention(files, "ply");
    let output_path = Path::new(&output_dir);
    std::fs::create_dir_all(output_path).expect("Failed to create output directory");
    let mut count = 0;

    let ply_parser = parser::Parser::<ply::DefaultElement>::new();
    let ply_writer = writer::Writer::new();
    for file_path in tqdm!(files_to_convert.into_iter()) {
        let mut f = std::fs::File::open(&file_path).unwrap();
        let mut ply = ply_parser.read_ply(&mut f).unwrap();

        println!("ply header: {:?}", ply.header);
        match storage_type {
            PCDDataType::Ascii => {
                ply.header.encoding = ply_rs::ply::Encoding::Ascii;
            },
            PCDDataType::Binary => {
                ply.header.encoding = set_encoding();
            },
            _ => unreachable!(),
        }
        let filename = Path::new(file_path.file_name().unwrap()).with_extension("ply");
        let output_file = output_path.join(filename);
        let mut file = File::create(&output_file).unwrap();

        if let Err(e) = ply_writer.write_ply(&mut file, &mut ply) {
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

fn pcd_to_pcd(output_dir:String, storage_type:PCDDataType, files:Vec<OsString>){
    let files_to_convert = filter_files_with_extention(files, "pcd");
    let output_path = Path::new(&output_dir);
    std::fs::create_dir_all(output_path).expect("Failed to create output directory");
    let mut count = 0;

    for file_path in tqdm!(files_to_convert.into_iter()) {
        // read one pcd file, determine type(ascii or binary)
        let pcd = read_pcd_file(file_path.clone()).unwrap();
        let filename = Path::new(file_path.file_name().unwrap()).with_extension("pcd");
        let output_file = output_path.join(filename);
        if let Err(e) = write_pcd_file(&pcd, storage_type, &output_file) {
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

fn expand_directory(p: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files = vec![];
    let dir_entry = p.read_dir().unwrap();
    for entry in dir_entry {
        let entry = entry.unwrap().path();
        if !entry.is_file() {
            // We do not recursively search
            continue;
        }

        if is_this_extension(&entry, extension) {
            files.push(entry);
        }
    }

    files
}

pub fn filter_files_with_extention(os_strings: Vec<OsString>, extension: &str) -> Vec<PathBuf> {
    let mut files_to_convert = vec![];
    for file_str in os_strings {
        let path = Path::new(&file_str);
        if path.is_dir() {
            files_to_convert.extend(expand_directory(path, extension));
        } else if is_this_extension(path, extension) {
            files_to_convert.push(path.to_path_buf());
        }
    }
    files_to_convert
}

fn is_this_extension(p: &Path, extension: &str) -> bool {
    p.extension().map(|f| extension.eq(f)).unwrap_or(false)
}


#[cfg(target_endian = "little")]
fn set_encoding() -> Encoding {
    Encoding::BinaryLittleEndian
}

#[cfg(target_endian = "big")]
fn set_encoding() -> Encoding {
    Encoding::BinaryBigEndian
}