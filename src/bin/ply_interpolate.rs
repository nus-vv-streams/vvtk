extern crate iswr;
// use std::env;
extern crate clap;
use clap::{Arg, App};
use iswr::materials::{ ply_file, points };
use std::io::{self, Write};
use std::path::{ PathBuf };

//example usage: cargo run  --bin ply_interpolate -- --unmapped
// the extra '--' after the binary file name is needed

fn main() {
     let matches = App::new("ply_interpolate")
     .about("Interpolate frame (t1) between 2 ply files (t0 & t2)")
     .arg(Arg::with_name("prev")
              .short("p")
              .long("prev")
              .takes_value(true)
              .multiple(false)
              .required(true)
              .help("File directory for frame t0"))
     .arg(Arg::with_name("next")
              .short("n")
              .long("next")
              .takes_value(true)
              .multiple(false)
              .required(true)
              .help("File directory for frame t1"))
     .arg(Arg::with_name("method")
              .short("m")
              .long("method")
              .takes_value(true)
              .multiple(false)
              .help("Method of interpolation"))
     .arg(Arg::with_name("coor_delta")
              .short("coor")
              .long("coor_delta")
              .takes_value(true)
              .multiple(false)
              .help("Weightage for coordinate delta in penalization function out of 100"))
     .arg(Arg::with_name("col_delta")
              .short("col")
              .long("col_delta")
              .takes_value(true)
              .multiple(false)
              .help("Weightage for colour delta in penalization function out of 100"))
     .arg(Arg::with_name("pre_mapped")
              .short("pm")
              .long("pre_mapped")
              .takes_value(true)
              .multiple(false)
              .help("Weightage for pre-mapped points in penalization function out of 100"))
    .arg(Arg::with_name("radius")
              .short("r")
              .long("radius")
              .takes_value(true)
              .multiple(false)
              .help("Radius for point desnity calculation"))
     .arg(Arg::with_name("nearest_points")
              .short("nearest_points")
              .long("nearest_points")
              .takes_value(true)
              .multiple(false)
              .help("Number of points extracted from kd-tree by distance before applying the penalization function"))
     .arg(Arg::with_name("unmapped")
              .short("u")
              .long("unmapped")
              .takes_value(false)
              .multiple(false)
              .help("Highlights unmapped points as green"))
     .arg(Arg::with_name("output")
              .short("o")
              .long("output")
              .takes_value(true)
              .multiple(false)
              .help("Output directory for interpolated frame / t2 with unmapped points highlighted"))

              
     .get_matches();


     let prev_frame_dir = matches.value_of("prev").unwrap();
     let next_frame_dir = matches.value_of("next").unwrap();   

     let method = matches.value_of("method").unwrap_or("closest_with_ratio_average_points_recovery");

     let show_unmapped_points = matches.is_present("unmapped");
     let output_dir = matches.value_of("output").unwrap_or("stdout");

     //  println!("show unmapped points: {}", show_unmapped_points);
    //  println!("interpolation method: {}", method);
     let coor_delta_weight = matches.value_of("coor_delta").unwrap_or("49.5").parse::<f32>().unwrap();
     let col_delta_weight = matches.value_of("col_delta").unwrap_or("49.5").parse::<f32>().unwrap();
     let pre_mapped_weight = matches.value_of("pre_mapped").unwrap_or("1").parse::<f32>().unwrap();
     let radius = matches.value_of("radius").unwrap_or("0.7").parse::<f32>().unwrap();
     let options_for_nearest = matches.value_of("nearest_points").unwrap_or("400").parse::<usize>().unwrap();

    interpolate(prev_frame_dir, next_frame_dir, method, coor_delta_weight, col_delta_weight, pre_mapped_weight, 
        radius, options_for_nearest, show_unmapped_points, output_dir);

     
}

fn interpolate(prev_frame_dir: &str, next_frame_dir: &str, method: &str, coor_delta_weight: f32, col_delta_weight: f32, 
    pre_mapped_weight: f32, radius: f32, options_for_nearest: usize, show_unmapped_points: bool, output_dir: &str)
{
    let mut prev = ply_file::PlyFile::new(prev_frame_dir).unwrap().read();
    let next = ply_file::PlyFile::new(next_frame_dir).unwrap().read();
    let mut result = points::Points::new();
    let mut reference_unmapped = points::Points::new();
    let mut marked_interpolated_frame = points::Points::new();

    if method == "closest_with_ratio_average_points_recovery"
    {
        let (result, reference_unmapped, marked_interpolated_frame) = prev.closest_with_ratio_average_points_recovery(&next, 
            coor_delta_weight/100.0, col_delta_weight/100.0, pre_mapped_weight/100.0, radius, options_for_nearest); //sum of first 3 must equal 1
    }


    //output block
    let output;
    if show_unmapped_points
    {
        output = reference_unmapped;
    }

    else
    {
        output = result;
    }

    if output_dir == "stdout"
    {
        //TODO: write to standard output using written_to_ascii
        // io::stdout().write_all(ply_to_ascii(*output));
    }

    else
    {
        iswr::materials::ply_file::PlyFile::create(output_dir).unwrap().writen_as_binary(output).unwrap();
    }

}

