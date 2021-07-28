#![allow(dead_code)]
mod materials;
pub mod methods;
mod tool;
mod traits;

use materials::{color, coordinate, ply_dir, ply_file, points};
#[allow(unused_imports)]
use methods::{change, filter};
use tool::renderer;

#[allow(unused_imports)]
use ply_dir::PlyDir;

fn main() {
    let path = "./plySource";
    // //frames are declared as mut since the delta is stored internally

    // // let max_coor: f32 = 3.0 * 512.0 * 512.0;
    // let scale_coor = 512.0 * 3.0_f32.sqrt(); //max_coor.sqrt();
    // let max_col: f32 = (100.0 * 100.0) + 2.0 * (256.0 * 256.0);
    // let scale_col = max_col.sqrt();

    let mut data_1051 = ply_file::PlyFile::new(&(path.to_owned() + "/longdress_vox10_1051.ply"))
        .unwrap()
        .read();
    let mut data_1053 = ply_file::PlyFile::new(&(path.to_owned() + "/longdress_vox10_1053.ply"))
        .unwrap()
        .read();

    let (a, reference, marked_interpolated_frame) = data_1051
        .closest_with_ratio_average_points_recovery(
            &data_1053,
            49.5 / 100.0,
            49.5 / 100.0,
            1.0 / 100.0,
            0.7,
            400,
            false,
            false,
            false,
            false,
        ); //sum of first 3 must equal 1

    a.render(); //complete interpolation and post processing
                // reference.render(); //reference frame with unmapped points marked as green
                // marked_interpolated_frame.render(); //interpolated frame with points surrounding cracks marked as red

    // let data_1051 = ply_file::PlyFile::new(&(path.to_owned() + "/longdress_vox10_1051.ply")).unwrap().read();
    // data_1051.seperate(sep_method::sep_by_y_coord).render_with_method(render_met::pt_size_2, render_met::all_red);
}
