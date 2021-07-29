#![allow(dead_code)]
pub mod materials;
pub mod methods;
pub mod tool;
pub mod traits;

#[allow(unused_imports)]
use methods::{filter, transform};
#[allow(unused_imports)]
use tool::renderer;

use materials::{color, coordinate, ply_dir, ply_file, points};

#[allow(unused_imports)]
use ply_dir::PlyDir;

fn main() {
    // let path = "/Users/hungkhoaitay/Documents/Hasagi/Ooi/in-summer-we-render/plySource/longdress/longdress/Ply";
    // //frames are declared as mut since the delta is stored internally

    // let data_1051 = ply_file::PlyFile::new(&(path.to_owned() + "/longdress_vox10_1223.ply")).read();
    // data_1051.seperate(sep_by_y_coord).render();
}

pub const NOTHING: &str = "nothing";
pub const OUT_DIR: &str = "plySource/out";
