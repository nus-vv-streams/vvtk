#![allow(dead_code)]
mod tool;
mod lib;
mod traits;

use tool::renderer;
use lib::{ color, coordinate, points, ply_file, ply_dir };
use ply_dir::PlyDir;
use points::Point;
use nalgebra::Point3;

use std::io;
use std::io::*;

fn main() 
{
    //frames are declared as mut since the delta is stored internally  

    let mut data_1051 = ply_file::PlyFile::new("plySource/longdress_vox10_1051.ply").read();

    let mut data_1053 = ply_file::PlyFile::new("plySource/longdress_vox10_1053.ply").read();

    println!("{}", "read frame 1053");

    // let c = data_1051.closest_with_ratio_average_points_recovery(&data_1061, 1.0);

//    let (mut a, reference) = data_1051.average_points_recovery(data_1053); //data_1053.clone().average_points_recovery(data_1051.clone());
    
    let (mut a, reference) = data_1051.closest_with_ratio_average_points_recovery(data_1053, 0.7, 0.1, 0.2); //sum must equal 1

    // let mut b = data_1051.average_points_recovery(data_1053);
    // b.render();

    // a.render();
    // a.data.append(&mut b.data);

   a.render();

   reference.render();

    ////////////////////////////////////

    // let mut delta_pos: Vec<Point3<f32>> = vec![];
    // let mut delta_col : Vec<Point3<f32>> = vec![];
    // delta_pos = data_1051.get_delta_pos_vector();
    // delta_col = data_1051.get_delta_colours();
    
    // for i in delta_pos.iter()
    // {
    //     println!("delta_coor: x pos : {}, y pos : {}, z pos: {}", i.x, i.y, i.z);
    // }

    // for i in delta_col.iter()
    // {
    //     println!("delta_col: x col : {}, y col : {}, z col: {}", i.x, i.y, i.z);
    // }
}
