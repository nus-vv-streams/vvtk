#![allow(dead_code)]
mod tool;
mod lib;
mod traits;

use tool::renderer;
use lib::{ color, coordinate, points, ply_file, ply_dir };
use ply_dir::PlyDir;
use points::Point;

fn main() {
    // plyDir::new("out").play();

    let data_1051 = ply_file::PlyFile::new("longdress_vox10_1051.ply").read();

    let data_1057 = ply_file::PlyFile::new("out/longdress_vox10_1061.ply").read();

    let c = data_1051.closest_with_ratio_average_points_recovery(&data_1057, 1.0);

    // let c = data_1051.average_points_recovery(&data_1057);

    c.render();

    // println!("{}", c.count());
}
