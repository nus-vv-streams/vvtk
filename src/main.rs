#![allow(dead_code)]
mod tool;
mod lib;
mod traits;
mod sep;
mod sep_method;

use tool::renderer;
use lib::{ color, coordinate, points, ply_file, ply_dir };
use sep_method::sep_by_y_coord;

#[allow(unused_imports)]
use ply_dir::PlyDir;

fn main() {
    let path = "/Users/hungkhoaitay/Documents/Hasagi/Ooi/in-summer-we-render/plySource/longdress/longdress/Ply";
    //frames are declared as mut since the delta is stored internally  

    let data_1051 = ply_file::PlyFile::new(&(path.to_owned() + "/longdress_vox10_1223.ply")).read();
    data_1051.seperate(sep_by_y_coord).render();
}
