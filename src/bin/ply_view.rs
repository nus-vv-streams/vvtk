#[macro_use]
extern crate error_chain;
extern crate iswr;
use clap::{App, Arg};
use iswr::{errors::*, ply_dir::PlyDir, reader};
// use std::io::{Error, ErrorKind};
use std::path::Path;

// cargo run --release --bin test | cargo run --release --bin ply_view -- --eye=100,100,100
quick_main!(run);

fn run() -> Result<()> {
    let matches = App::new("ply_view")
        .about("View a ply frame or play a ply video")
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
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

    let eye = match matches.values_of("eye") {
        Some(vec) => Some(
            Some(vec.collect::<Vec<_>>())
                .filter(|vec| vec.len() == 3)
                .map(|vec| process(vec))
                .chain_err(|| "Inappropriate number of arguments in eye")?
                .chain_err(|| "Inappropriate type of arguments, should be float number")?,
        ),
        None => None,
    };

    let at = match matches.values_of("at") {
        Some(vec) => Some(
            Some(vec.collect::<Vec<_>>())
                .filter(|vec| vec.len() == 3)
                .map(|vec| process(vec))
                .chain_err(|| "Inappropriate number of arguments in at")?
                .chain_err(|| "Inappropriate type of arguments, should be float number")?,
        ),
        None => None,
    };

    let input = matches.value_of("input");

    match input {
        Some(path) => {
            let new_path = Path::new(&path);
            if new_path.is_file() {
                reader::read(input)
                    .chain_err(|| "Problem with the input")?
                    .render_with_camera(eye, at);
            } else if new_path.is_dir() {
                PlyDir::new(&path).play_with_camera(eye, at);
            } else {
                print!("No such file or dir {}", path)
            }
        }
        None => {
            iswr::reader::read(input)
                .chain_err(|| "Problem with the input")?
                .render_with_camera(eye, at);
        }
    };

    Ok(())
}

fn process(vec: Vec<&str>) -> Result<nalgebra::Point3<f32>> {
    Ok(nalgebra::Point3::new(
        vec[0].parse::<f32>()?,
        vec[1].parse::<f32>()?,
        vec[2].parse::<f32>()?,
    ))
}
