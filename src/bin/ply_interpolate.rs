#[macro_use]
extern crate error_chain;
extern crate vivotk;
// use std::env;
extern crate clap;
use clap::{App, Arg};
use vivotk::errors::*;
use vivotk::io::{reader, writer};
use vivotk::pointcloud::PointCloud;
use vivotk::processing::conceal::concealed_pointcloud::ConcealedPointCloud;
use vivotk::processing::conceal::interpolate::*;
use vivotk::processing::conceal::interpolate_params::InterpolateParams;

// use std::path::{ PathBuf };

// example usage: cargo ply_interpolate --unmapped
quick_main!(run);

fn run() -> Result<()> {
    let matches = App::new("ply_interpolate")
     .about("Interpolate frame (t1) between 2 ply files (t0 & t2)")
     .arg(Arg::with_name("prev")
              .short('p')
              .long("prev")
              .takes_value(true)
              .multiple(false)
              .required(true)
              .help("File directory for frame t0"))
     .arg(Arg::with_name("next")
              .short('n')
              .long("next")
              .takes_value(true)
              .multiple(false)
              .required(true)
              .help("File directory for frame t1"))
     .arg(Arg::with_name("method")
              .long("method")
              .takes_value(true)
              .multiple(false)
              .help("Method of interpolation"))
     .arg(Arg::with_name("two_way")
              .long("two_way")
              .takes_value(false)
              .multiple(false)
              .help("Interpolate t0 as reference with t2, t2 as reference with t0 and concatenate the results"))
     .arg(Arg::with_name("coor_delta")
              .long("coor_delta")
              .takes_value(true)
              .multiple(false)
              .help("Weightage for coordinate delta in penalization function out of 100"))
     .arg(Arg::with_name("scale_coor_delta")
              .long("scale_coor_delta")
              .takes_value(true)
              .multiple(false)
              .help("Scale factor to make the coordinate delta within [0, 1]"))
     .arg(Arg::with_name("col_delta")
              .long("col_delta")
              .takes_value(true)
              .multiple(false)
              .help("Weightage for colour delta in penalization function out of 100"))
     .arg(Arg::with_name("prev_weight")
              .long("prev_weight")
              .takes_value(true)
              .multiple(false)
              .help("Weight for previous frame when averaging points to get interpolated point of of 100; next_weight is automatically set to 1 - prev_weight/100"))
     .arg(Arg::with_name("scale_col_delta")
              .long("scale_col_delta")
              .takes_value(true)
              .multiple(false)
              .help("Scale factor to make the color delta within [0, 1]"))
     .arg(Arg::with_name("pre_mapped")
              .long("pre_mapped")
              .takes_value(true)
              .multiple(false)
              .help("Weightage for pre-mapped points in penalization function out of 100"))
    .arg(Arg::with_name("density_radius")
              .long("density_radius")
              .takes_value(true)
              .multiple(false)
              .help("Radius for point desnity calculation"))
     .arg(Arg::with_name("nearest_points")
              .long("nearest_points")
              .takes_value(true)
              .multiple(false)
              .help("Number of points extracted from kd-tree by distance before applying the penalization function"))
     .arg(Arg::with_name("unmapped")
              .long("unmapped")
              .takes_value(false)
              .multiple(false)
              .help("Highlights unmapped points as green"))
     .arg(Arg::with_name("resize")
              .long("resize")
              .takes_value(false)
              .multiple(false)
              .help("Increases size of points near cracks to 2.0 based on point density"))
     .arg(Arg::with_name("dist_func")
              .long("dist_func")
              .takes_value(true)
              .multiple(false)
              .help("Define which distance fucnction to use"))
     .arg(Arg::with_name("mark_enlarged")
              .long("mark_enlarged")
              .takes_value(false)
              .multiple(false)
              .help("Highlights enlarged points as red"))
    .arg(Arg::with_name("threads")
              .short('t')
              .long("threads")
              .takes_value(true)
              .multiple(false)
              .help("Number of threads used for interpolation"))
     .arg(Arg::with_name("frame_delta")
              .long("frame_delta")
              .takes_value(false)
              .multiple(false)
              .help("Computes delta of coordinates and colour between interpolated frame and t2"))
     .arg(Arg::with_name("output")
              .short('o')
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

    let mut params: InterpolateParams = InterpolateParams::new();
    params.show_unmapped_points = matches.is_present("unmapped");
    params.mark_enlarged = matches.is_present("mark_enlarged");
    params.compute_frame_delta = matches.is_present("frame_delta");
    params.resize_near_cracks = matches.is_present("resize");
    let two_way_interpolation = matches.is_present("two_way");

    let output_dir = matches.value_of("output");
    let exists_output_dir = matches.is_present("output");

    //  println!("show unmapped points: {}", show_unmapped_points);
    //  println!("interpolation method: {}", method);

    let dist_func_name = matches.value_of("dist_func").unwrap_or("inf_norm");
    let dist_func: for<'r, 's> fn(&'r [f32; 3], &'s [f32; 3]) -> f32;
    match dist_func_name {
        "inf_norm" => dist_func = inf_norm,
        "two_norm" => dist_func = two_norm,
        _ => dist_func = inf_norm,
    }
    params.dist_func = dist_func;

    params.prev_weight = matches
        .value_of("prev_weight")
        .unwrap_or("50")
        .parse::<f32>()
        .unwrap()
        / 100.0;
    if params.prev_weight > 1.0 {
        panic!("Entered prev_weight value exceeds 100");
    }
    params.next_weight = 1.0 - params.prev_weight;

    params.scale_coor_delta = matches
        .value_of("scale_coor_delta")
        .unwrap_or("1010.0")
        .parse::<f32>()
        .unwrap();

    params.scale_col_delta = matches
        .value_of("scale_col_delta")
        .unwrap_or("256.0")
        .parse::<f32>()
        .unwrap();

    params.threads = matches
        .value_of("threads")
        .unwrap_or("1")
        .parse::<usize>()
        .unwrap();

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
    params.density_radius = matches
        .value_of("density_radius")
        .unwrap_or("2.0")
        .parse::<f32>()
        .unwrap();
    params.neighborhood_size = matches
        .value_of("nearest_points")
        .unwrap_or("10")
        .parse::<usize>()
        .unwrap();

    interpolate(
        prev_frame_dir,
        next_frame_dir,
        method,
        two_way_interpolation,
        params,
        output_dir,
        exists_output_dir,
    )
}

fn interpolate(
    prev_frame_dir: Option<&str>,
    next_frame_dir: Option<&str>,
    method: &str,
    two_way_interpolation: bool,
    params: InterpolateParams,
    output_dir: Option<&str>,
    exists_output_dir: bool,
) -> Result<()> {
    let prev = reader::read(prev_frame_dir)
        .chain_err(|| "Problem with the input of prev frame")?
        .get_points();
    let next = reader::read(next_frame_dir)
        .chain_err(|| "Problem with the input of next frame")?
        .get_points();

    let mut end_result = PointCloud::new();

    let prev_pc = ConcealedPointCloud::new_from_point_cloud(prev);
    let next_pc = ConcealedPointCloud::new_from_point_cloud(next);

    if method == "closest_with_ratio_average_points_recovery" {
        if two_way_interpolation {
            let (_interpolated_pc, prev_pc, next_pc) =
                closest_with_ratio_average_points_recovery(prev_pc, next_pc, params.clone()); //sum of first 3 must equal 1

            let (mut interpolated_pc, _prev_pc, _next_pc) =
                closest_with_ratio_average_points_recovery(prev_pc, next_pc, params.clone()); //sum of first 3 must equal 1

            end_result.data.append(&mut interpolated_pc.pc.data);
        } else {
            let (mut interpolated_pc, _prev_pc, _next_pc) =
                closest_with_ratio_average_points_recovery(prev_pc, next_pc, params.clone()); //sum of first 3 must equal 1

            end_result.data.append(&mut interpolated_pc.pc.data);
        }
    }

    let output;

    //output block

    output = end_result;

    if !exists_output_dir {
        // output
        //     .write(None, None)
        //     .chain_err(|| "Problem with the output")?;

        writer::write(output, None, None).chain_err(|| "Problem with the output")?;
    } else {
        // output
        //     .write(None, output_dir)
        //     .chain_err(|| "Problem with the output")?;

        writer::write(output, None, output_dir).chain_err(|| "Problem with the output")?;
    }

    Ok(())
}
