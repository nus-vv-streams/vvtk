use std::str::FromStr;
use std::fs::File;
use std::{path::{Path, PathBuf}, ffi::OsString};
use kdam::tqdm;
use clap::{Parser};
use ply_rs::{ply, parser, writer, ply::{Encoding, Payload}, ply::DefaultElement};

use vivotk::pcd::{write_pcd_file, read_pcd_file, PCDDataType};
use vivotk::ply::read_ply;
use vivotk::formats::pointxyzrgba::PointXyzRgba;
use vivotk::formats::PointCloud;
use vivotk::pipeline::subcommands::write::create_pcd;
use vivotk::pipeline::subcommands::to_png::{pc_to_png, ToPng};
use vivotk::utils::{find_all_files, read_file_to_point_cloud};

mod ply_play_mod;

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
    #[clap(short = 'n', long)]
    frames: Option<usize>,
    #[clap(short = 'x', long, default_value_t = 0.0)]
    camera_x: f32,
    #[clap(short = 'y', long, default_value_t = 0.0)]
    camera_y: f32,
    #[clap(short = 'z', long, default_value_t = 1.3)]
    camera_z: f32,
    #[clap(long = "yaw", default_value_t = -90.0, allow_hyphen_values = true)]
    camera_yaw: f32,
    #[clap(long = "pitch", default_value_t = 0.0)]
    camera_pitch: f32,
    #[clap(long, default_value_t = 1600)]
    width: u32,
    #[clap(long, default_value_t = 900)]
    height: u32,
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
    let cmd_args_original = std::env::args_os();
    let cmd_args_vec: Vec<String> = cmd_args_original
        .map(|arg| arg.into_string().unwrap())
        .collect(); 
    println!("cmd_args_vec: {:?}", cmd_args_vec);
    let subcommand: VVSubCommand = VVSubCommand::parse_from(cmd_args_vec.clone());
    match subcommand {
        VVSubCommand::Convert(args) => {
            println!("Convert mode, rest of args: {:?} ", args);
            let mut files = find_all_files(&args.input);
            files.sort();
            for file in tqdm!(files.into_iter()) {
                println!("file: {:?}", file);
                let current_file_type = file.extension().unwrap().to_str().unwrap();
                let target_file_type = args.output_format.to_string();

                // create output dir
                let output_path = Path::new(&args.output);
                std::fs::create_dir_all(output_path).expect("Failed to create output directory");
                match (current_file_type, target_file_type.as_str()) {
                    ("ply", "ply") => ply_to_ply(output_path, args.storage_type, file),
                    ("ply", "pcd") => ply_to_pcd(output_path, args.storage_type, file),
                    ("pcd", "ply") => pcd_to_ply(output_path, args.storage_type, file),
                    ("pcd", "pcd") => pcd_to_pcd(output_path, args.storage_type, file),
                    (_, "png")     => {
                        convert_to_png(file, &args)
                    }
                    _ => println!("unsupported file type"),
                }
            }
        }
        VVSubCommand::Play(args) => {
            println!("play mode, rest of args: {:?} ", args);
            ply_play_mod::play(args);
        }
    }
}

fn convert_to_png(file_path:PathBuf, args: &ConvertArgs){
    let pc = read_file_to_point_cloud(&file_path);
    // println!("args_vec: {:?}", args);
    let args_vec: Vec<String> = [ "to_png",
        "--output-dir", args.output.as_str(),
        "-x", args.camera_x.to_string().as_str(),
        "-y", args.camera_y.to_string().as_str(),
        "-z", args.camera_z.to_string().as_str(),
        "--yaw", args.camera_yaw.to_string().as_str(),
        "--pitch", args.camera_pitch.to_string().as_str(),
        "--width", args.width.to_string().as_str(),
        "--height", args.height.to_string().as_str(),
    ].iter().map(|s| s.to_string()).collect();
    let mut to_png = ToPng::from_args_unboxed(args_vec); 
    let filename = Path::new(file_path.file_name().unwrap());
    pc_to_png(&mut to_png, pc.unwrap(), filename.to_str().unwrap());
}

fn ply_to_ply(output_path:&Path, storage_type:PCDDataType, file_path:PathBuf){
    let ply_parser = parser::Parser::<ply::DefaultElement>::new();
    let mut f = std::fs::File::open(&file_path).unwrap();
    let mut ply = ply_parser.read_ply(&mut f).unwrap();

    ply.header.encoding = match storage_type {
        PCDDataType::Ascii => ply_rs::ply::Encoding::Ascii,
        PCDDataType::Binary => set_encoding(),
        _ => unreachable!(),
    };
        
    let filename = Path::new(file_path.file_name().unwrap()).with_extension("ply");
    let output_file = output_path.join(filename);
    let mut file = File::create(&output_file).unwrap();

    let ply_writer = writer::Writer::<ply::DefaultElement>::new();
    if let Err(e) = ply_writer.write_ply(&mut file, &mut ply) {
        println!(
            "Failed to write {:?} to {:?}\n{e}",
            file_path.into_os_string(),
            output_file.into_os_string()
        );
    }

}

fn pcd_to_pcd(output_path:&Path, storage_type:PCDDataType, file_path:PathBuf){
    let pcd = read_pcd_file(file_path.clone()).unwrap();
    let filename = Path::new(file_path.file_name().unwrap()).with_extension("pcd");
    let output_file = output_path.join(filename);
    if let Err(e) = write_pcd_file(&pcd, storage_type, &output_file) {
        println!(
            "Failed to write {:?} to {:?}\n{e}",
            file_path.into_os_string(),
            output_file.into_os_string()
        );
    }
}

fn ply_to_pcd(output_path:&Path, storage_type:PCDDataType, file_path:PathBuf){
    let pointxyzrgba = read_ply(file_path.clone()).unwrap();
    let pcd = create_pcd(&pointxyzrgba);

    let filename = Path::new(file_path.file_name().unwrap()).with_extension("pcd");
    let output_file = output_path.join(filename.clone());
    if let Err(e) = write_pcd_file(&pcd, storage_type, &output_file) {
        println!(
            "Failed to write {:?} to {:?}\n{e}",
            file_path.into_os_string(),
            output_file.into_os_string()
        );
    }
}


fn pcd_to_ply(output_path:&Path, storage_type:PCDDataType, file_path:PathBuf){
    let pcd = read_pcd_file(&file_path).unwrap();

    let x_prop_def = ply_rs::ply::PropertyDef::new("x".to_string(), ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::Float));
    let y_prop_def = ply_rs::ply::PropertyDef::new("y".to_string(), ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::Float));
    let z_prop_def = ply_rs::ply::PropertyDef::new("z".to_string(), ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::Float));
    let red_prop_def = ply_rs::ply::PropertyDef::new("red".to_string(), ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::UChar));
    let green_prop_def = ply_rs::ply::PropertyDef::new("green".to_string(), ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::UChar));
    let blue_prop_def = ply_rs::ply::PropertyDef::new("blue".to_string(), ply_rs::ply::PropertyType::Scalar(ply_rs::ply::ScalarType::UChar));
    
    let mut element = ply_rs::ply::ElementDef::new("vertex".to_string());
    element.properties.insert("x".to_string(), x_prop_def);
    element.properties.insert("y".to_string(), y_prop_def);
    element.properties.insert("z".to_string(), z_prop_def);
    element.properties.insert("red".to_string(), red_prop_def);
    element.properties.insert("green".to_string(), green_prop_def);
    element.properties.insert("blue".to_string(), blue_prop_def);
    element.count = pcd.header().width() as usize;

    let mut ply_header = ply_rs::ply::Header::new();
    ply_header.encoding = match storage_type {
        PCDDataType::Ascii => ply_rs::ply::Encoding::Ascii,
        PCDDataType::Binary => set_encoding(),
        _ => unreachable!(),
    };
    ply_header.elements.insert("vertex".to_string(), element); 

    let pcd_pointxyzrgba: PointCloud<PointXyzRgba> = pcd.into();
    let mut pay_load_vec = Vec::<DefaultElement>::new();
    pcd_pointxyzrgba.points.into_iter().for_each(|point| {
        let mut ply_point = DefaultElement::new();
        ply_point.insert("x".to_string(), ply_rs::ply::Property::Float(point.x));
        ply_point.insert("y".to_string(), ply_rs::ply::Property::Float(point.y));
        ply_point.insert("z".to_string(), ply_rs::ply::Property::Float(point.z));
        ply_point.insert("red".to_string(), ply_rs::ply::Property::UChar(point.r));
        ply_point.insert("green".to_string(), ply_rs::ply::Property::UChar(point.g));
        ply_point.insert("blue".to_string(), ply_rs::ply::Property::UChar(point.b));
        pay_load_vec.push(ply_point);
    });
    let mut pay_load = Payload::<DefaultElement>::new();
    pay_load.insert("vertex".to_string(), pay_load_vec);
    
    let mut ply = ply_rs::ply::Ply::<DefaultElement>::new();
    ply.header = ply_header;
    ply.payload = pay_load;

    let filename = Path::new(file_path.file_name().unwrap()).with_extension("ply");
    let output_file = output_path.join(filename);
    let mut file = File::create(&output_file).unwrap();

    let ply_writer = writer::Writer::<ply::DefaultElement>::new();
    if let Err(e) = ply_writer.write_ply(&mut file, &mut ply) {
        println!(
            "Failed to write {:?} to {:?}\n{e}",
            file_path.into_os_string(),
            output_file.into_os_string()
        );
    }

}



#[cfg(target_endian = "little")]
fn set_encoding() -> Encoding {
    Encoding::BinaryLittleEndian
}

#[cfg(target_endian = "big")]
fn set_encoding() -> Encoding {
    Encoding::BinaryBigEndian
}