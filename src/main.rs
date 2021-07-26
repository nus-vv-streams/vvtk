#![allow(dead_code)]
mod tool;
mod traits;
mod materials;
pub mod methods;

use tool::{ renderer };
#[allow(unused_imports)]
use methods::{ sep_method, render_met };
use materials::{ color, coordinate, points, ply_file, ply_dir, sep };
use std::io::{self, Write};

#[allow(unused_imports)]
use ply_dir::PlyDir;

fn main() {
    let path = "plySource/binary_ply";
    // //frames are declared as mut since the delta is stored internally  

    // let mut data_1051 = ply_file::PlyFile::new(&(path.to_owned() + "/longdress_vox10_1051.ply")).unwrap().read();
    // let mut data_1053 = ply_file::PlyFile::new(&(path.to_owned() + "/longdress_vox10_1053.ply")).unwrap().read();
    // let (a, reference, marked_interpolated_frame) = data_1051.closest_with_ratio_average_points_recovery(data_1053, 0.495, 0.495, 0.01, 0.7); //sum of first 3 must equal 1

    // a.render(); //comeplete interpolation and post processing
    // reference.render(); //reference frame with unmapped points marked as green
    // marked_interpolated_frame.render(); //interpolated frame with points surrounding cracks marked as red

    let points = ply_file::PlyFile::new(&(path.to_owned() + "/longdress_vox10_1051.ply")).unwrap().read();
    ply_file::PlyFile::create("path").unwrap().writen_as_ascii(points).unwrap();
    // io::stdout().write_all(points).unwrap();
}
