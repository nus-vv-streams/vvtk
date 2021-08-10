#[macro_use]
extern crate error_chain;
extern crate iswr;
// use std::env;
extern crate clap;
use clap::{App, Arg};
use iswr::{errors::*, params::Params, points, reader};
// use std::io::{self, Write};

// use std::path::{ PathBuf };

// example usage: cargo run  --bin ply_interpolate -- --unmapped
// the extra '--' after the binary file name is needed
quick_main!(run);

fn run() -> Result<()> {
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
     .arg(Arg::with_name("two_way")
              .short("two_way")
              .long("two_way")
              .takes_value(false)
              .multiple(false)
              .help("Interpolate t0 as reference with t2, t2 as reference with t0 and concatenate the results"))
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
     .arg(Arg::with_name("resize")
              .short("resize")
              .long("resize")
              .takes_value(false)
              .multiple(false)
              .help("Increases size of points near cracks to 2.0 based on point density"))
     .arg(Arg::with_name("mark_resized")
              .short("mark_resized")
              .long("mark_resized")
              .takes_value(false)
              .multiple(false)
              .help("Highlights enlarged points as red"))
     .arg(Arg::with_name("frame_delta")
              .short("frame_delta")
              .long("frame_delta")
              .takes_value(false)
              .multiple(false)
              .help("Computes delta of coordinates and colour between interpolated frame and t2"))
     .arg(Arg::with_name("output")
              .short("o")
              .long("output")
              .takes_value(true)
              .multiple(false)
              .help("Output directory for interpolated frame / t2 with unmapped points highlighted"))
     .get_matches();

    let prev_frame_dir = matches.value_of("prev");
    let next_frame_dir = matches.value_of("next");

    let method = matches
        .value_of("method")
        .unwrap_or("closest_with_ratio_average_points_recovery");

    let mut params: Params = Params::new();
    params.show_unmapped_points = matches.is_present("unmapped");
    params.mark_enlarged = matches.is_present("mark_enlarged");
    params.compute_frame_delta = matches.is_present("frame_delta");
    params.resize_near_cracks = matches.is_present("resize");
    let two_way_interpolation = matches.is_present("two_way");

    let output_dir = matches.value_of("output");

    //  println!("show unmapped points: {}", show_unmapped_points);
    //  println!("interpolation method: {}", method);
    params.penalize_coor = matches
        .value_of("coor_delta")
        .unwrap_or("49.5")
        .parse::<f32>()
        .unwrap()
        / 100.0;
    params.penalize_col = matches
        .value_of("col_delta")
        .unwrap_or("49.5")
        .parse::<f32>()
        .unwrap()
        / 100.0;
    params.penalize_mapped = matches
        .value_of("pre_mapped")
        .unwrap_or("1")
        .parse::<f32>()
        .unwrap()
        / 100.0;
    params.radius = matches
        .value_of("radius")
        .unwrap_or("0.7")
        .parse::<f32>()
        .unwrap();
    params.options_for_nearest = matches
        .value_of("nearest_points")
        .unwrap_or("400")
        .parse::<usize>()
        .unwrap();

    interpolate(
        prev_frame_dir,
        next_frame_dir,
        method,
        two_way_interpolation,
        params,
        output_dir,
    )
}

fn interpolate(
    prev_frame_dir: Option<&str>,
    next_frame_dir: Option<&str>,
    method: &str,
    two_way_interpolation: bool,
    params: Params,
    output_dir: Option<&str>,
) -> Result<()> {
    let mut prev =
        reader::read(prev_frame_dir).chain_err(|| "Problem with the input of prev frame")?;
    let mut next =
        reader::read(next_frame_dir).chain_err(|| "Problem with the input of next frame")?;

    let mut end_result = points::Points::new();
    let mut end_reference_unmapped = points::Points::new();
    let mut end_marked_interpolated_frame = points::Points::new();

    if method == "closest_with_ratio_average_points_recovery" {
        if two_way_interpolation {
            let (mut prev_result, _reference_unmapped, _marked_interpolated_frame) =
                prev.closest_with_ratio_average_points_recovery(next.clone(), params.clone()); //sum of first 3 must equal 1

            let (mut result, reference_unmapped, marked_interpolated_frame) =
                next.closest_with_ratio_average_points_recovery(prev, params.clone()); //sum of first 3 must equal 1

            result.data.append(&mut prev_result.data);
            end_result = result;
            end_reference_unmapped = reference_unmapped;
            end_marked_interpolated_frame = marked_interpolated_frame;
        } else {
            let (result, reference_unmapped, marked_interpolated_frame) =
                prev.closest_with_ratio_average_points_recovery(next, params.clone()); //sum of first 3 must equal 1

            end_result = result;
            end_reference_unmapped = reference_unmapped;
            end_marked_interpolated_frame = marked_interpolated_frame;
        }
    }

    let output;

    //output block

    if params.show_unmapped_points {
        output = end_reference_unmapped;
    } else if params.mark_enlarged {
        output = end_marked_interpolated_frame;
    } else {
        output = end_result;
    }

    output
        .write(None, output_dir)
        .chain_err(|| "Problem with the output")?;

    Ok(())
}
