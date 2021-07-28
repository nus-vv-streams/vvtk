extern crate iswr;
use clap::{App, Arg};
use std::path::Path;

fn main() {
    let matches = App::new("ply_view")
        .about("View a ply frame or play a ply video")
        .arg(
            Arg::with_name("source")
                .short("s")
                .long("source")
                .takes_value(true)
                .multiple(false)
                .help("File directory for data"),
        )
        .arg(
            Arg::with_name("eye")
                .long("eye")
                .use_delimiter(true)
                .takes_value(true)
                .multiple(false)
                .help("Position of eye"),
        )
        .arg(
            Arg::with_name("at")
                .long("at")
                .use_delimiter(true)
                .takes_value(true)
                .multiple(false)
                .help("Position of at"),
        )
        .get_matches();

    let eye_vec = matches
        .values_of("eye")
        .unwrap_or_default()
        .collect::<Vec<_>>();
    let at_vec = matches
        .values_of("at")
        .unwrap_or_default()
        .collect::<Vec<_>>();
    // let eye = nalgebra::Point3::new(eye_vec[0], eye_vec[1], eye_vec[2]);
    // let at = nalgebra::Point3::new(at_vec[0], at_vec[1], at_vec[2]);

    let eye = if eye_vec.len() >= 3 {
        Some(nalgebra::Point3::new(
            eye_vec[0].parse::<f32>().unwrap(),
            eye_vec[1].parse::<f32>().unwrap(),
            eye_vec[2].parse::<f32>().unwrap(),
        ))
    } else {
        println!("Not enough argument for eye, using default");
        None
    };

    let at = if at_vec.len() >= 3 {
        Some(nalgebra::Point3::new(
            at_vec[0].parse::<f32>().unwrap(),
            at_vec[1].parse::<f32>().unwrap(),
            at_vec[2].parse::<f32>().unwrap(),
        ))
    } else {
        println!("Not enough argument for at, using default");
        None
    };

    let source = matches.value_of("source").unwrap();

    let new_path = Path::new(&source);
    if new_path.is_file() {
        iswr::materials::ply_file::PlyFile::new(&source)
            .unwrap()
            .render_with_camera(eye, at);
    } else if new_path.is_dir() {
        iswr::materials::ply_dir::PlyDir::new(&source).play_with_camera(eye, at);
    } else {
        print!("No such file or dir {}", source)
    }
}
