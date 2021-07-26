#![allow(dead_code)]
pub mod tool;
pub mod materials;
pub mod traits;
pub mod methods;

#[allow(unused_imports)]
use tool::{ renderer };
#[allow(unused_imports)]
use methods::{ sep_method, render_met };

use materials::{ color, coordinate, points, ply_file, ply_dir, sep };

#[allow(unused_imports)]
use ply_dir::PlyDir;

use std::env;
// use std::error::Error;
// use std::path::{ PathBuf };

fn main() {
    // let path = "/Users/hungkhoaitay/Documents/Hasagi/Ooi/in-summer-we-render/plySource/longdress/longdress/Ply";
    // //frames are declared as mut since the delta is stored internally  

    // let data_1051 = ply_file::PlyFile::new(&(path.to_owned() + "/longdress_vox10_1223.ply")).read();
    // data_1051.seperate(sep_by_y_coord).render();

    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
}

pub const NOTHING: &str = "nothing";
pub const OUT_DIR: &str = "plySource/out";
// pub const OUT_DIR_BUF: &PathBuf = &PathBuf::from(OUT_DIR);

pub struct Config {
    pub filename1: String,
    pub filename2: String,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &str> {
        let len = args.len();
        if len > 3 {
            Err("too many arguments")
        } else if len == 3 {
            let filename1 = args[1].clone();
            let filename2 = args[2].clone();

            Ok(Config { filename1, filename2 })
        } else if len == 2 {
            Config::new_with_one_arg(args)
        } else {
            Err("no arguments")
        }
    }

    pub fn new_with_one_arg(args: &[String]) -> Result<Config, &str> {
        if args.len() > 2 {
            print!("{:?}", args);
            return Err("too many arguments");
        }

        let filename1 = args[1].clone();

        Ok(Config { filename1, filename2: NOTHING.to_string() })
    }
}

// pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
//     // --snip--
// }

