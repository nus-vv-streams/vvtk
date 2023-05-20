// use clap::Parser;
use kdam::tqdm;
// use ply_rs::{ply, ply::Property};
// use std::ffi::OsString;
use std::{path::{Path, PathBuf}, ffi::OsString};
// use vivotk::pcd::{
//     write_pcd_file, PCDDataType, PCDField, PCDFieldSize, PCDFieldType, PCDHeader, PCDVersion,
//     PointCloudData,
// };
use vivotk::pcd::{PCDDataType};
use clap::{command, Command, arg, ArgAction};

mod ply_to_pcd_mod;
use ply_to_pcd_mod::ply_to_pcd;

use vivotk::pcd::{read_pcd, read_pcd_file};


fn main() {
    // print hello world
    println!("Hello, world start!");
    let matches = command!() // requires `cargo` feature
        .subcommand(
            Command::new("test")
                .about("does testing things")
                .arg(arg!(-l --list "lists test values").action(ArgAction::SetFalse)),
        )
        .subcommand(
            Command::new("convert")
                .about("does convert things")
                .arg(
                    arg!(--input  <INPUT_DIR>  "specify input directory")
                    .required(true).action(clap::ArgAction::Append)
                    
                )
                .arg(
                    arg!(--output <OUTPUT_DIR> "specify output directory")
                    .required(true)
                )
                .arg(
                    arg!(--out_format <OUTPUT_FORMAT> "specify output format, can be ply or pcd")
                    .default_value("pcd")
                    .required(false)
                )
                .arg(
                    arg!(--storage_type <STORAGE_TYPE> "specify output type, can be ascii or binary")
                    .default_value("binary")
                    .required(false)
                )
                , 
        )
        .subcommand(
            Command::new("play")
                .about("does play things")
        )
        .get_matches();

        match matches.subcommand() {
            Some(("convert", sub_matches)) => {
                // println!("subcommand matches: {:?}", sub_matches);
                let input_dirs: Vec<OsString> = sub_matches.get_many::<String>("input")
                                                        .unwrap_or_default().map(|x| x.clone().into()).collect();

                println!("input_dir: {:?}", &input_dirs);
                let output_dir = sub_matches.get_one::<String>("output").unwrap();
                println!("output_dir: {:?}", output_dir);
                let out_format = sub_matches.get_one::<String>("out_format").unwrap();
                println!("out_format: {:?}", out_format);
                let storage_type = sub_matches.get_one::<String>("storage_type").unwrap();
                
                // convert to type Vec<OsString>
                let storage_pcd_data_type: PCDDataType = storage_type.parse().unwrap();
                println!("storage_type_PCDDataType: {:?}", storage_pcd_data_type); 
                 
                // ply_to_pcd(output_dir.clone(), storage_pcd_data_type, input_dirs.clone());

                pcd_to_pcd(output_dir.clone(), storage_pcd_data_type, input_dirs.clone());           


            }
            Some(("play", sub_matches)) => {
                println!("play");
                println!("subcommand matches: {:?}", sub_matches);
            }
            _ => unreachable!(),
        }

    println!("Hello, world end!");
}

fn pcd_to_pcd(output_dir:String, storage_type:PCDDataType, files:Vec<OsString>){
    let files_to_convert = filter_for_pcd_files(files);
    let output_path = Path::new(&output_dir);
    std::fs::create_dir_all(output_path).expect("Failed to create output directory");
    let mut count = 0;

    'outer: for file_path in tqdm!(files_to_convert.into_iter()) {
        // read one pcd file, determine type(ascii or binary)
        println!("file_path: {:?}", file_path.clone());
        let res = read_pcd_file(file_path.clone()).unwrap();
        let header = res.header();
        println!("header: {:?}", header);
        break;
    }
}


fn is_pcd_file(p: &Path) -> bool {
    p.extension().map(|f| "pcd".eq(f)).unwrap_or(false)
}

pub fn filter_for_pcd_files(os_strings: Vec<OsString>) -> Vec<PathBuf> {
    let mut files_to_convert = vec![];
    for file_str in os_strings {
        let path = Path::new(&file_str);
        if path.is_dir() {
            files_to_convert.extend(expand_directory(path));
        } else if is_pcd_file(path) {
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

        if is_pcd_file(&entry) {
            ply_files.push(entry);
        }
    }

    ply_files
}