#[macro_use]
extern crate error_chain;
extern crate iswr;
use clap::{App, Arg};
use iswr::{errors::*, reader};

// cargo run --release --bin test | cargo run --release --bin ply_view -- --eye=100,100,100
quick_main!(run);

fn run() -> Result<()> {
    let matches = App::new("ply_to_png")
        .about("Make a png from ply file")
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
        .arg(
            Arg::with_name("x")
                .short("x")
                .long("x")
                .use_delimiter(true)
                .takes_value(true)
                .multiple(false)
                .help("x coordinate of bottom left corner"),
        )
        .arg(
            Arg::with_name("y")
                .short("y")
                .long("y")
                .use_delimiter(true)
                .takes_value(true)
                .multiple(false)
                .help("Position of at"),
        )
        .arg(
            Arg::with_name("width")
                .short("w")
                .long("width")
                .use_delimiter(true)
                .takes_value(true)
                .multiple(false)
                .help("Width of PNG file"),
        )
        .arg(
            Arg::with_name("height")
                .short("h")
                .long("height")
                .use_delimiter(true)
                .takes_value(true)
                .multiple(false)
                .help("Height of PNG file"),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .takes_value(true)
                .multiple(false)
                .required(true)
                .help("File directory for output"),
        )
        .get_matches();

    let eye = match matches.values_of("eye") {
        Some(vec) => Some(
            Some(vec.collect::<Vec<_>>())
                .filter(|vec| vec.len() == 3)
                .map(process)
                .chain_err(|| "Inappropriate number of arguments in eye")?
                .chain_err(|| "Inappropriate type of arguments, should be float number")?,
        ),
        None => None,
    };

    let at = match matches.values_of("at") {
        Some(vec) => Some(
            Some(vec.collect::<Vec<_>>())
                .filter(|vec| vec.len() == 3)
                .map(process)
                .chain_err(|| "Inappropriate number of arguments in at, need 3 arguments")?
                .chain_err(|| "Inappropriate type of arguments, should be float number {}")?,
        ),
        None => None,
    };

    let x = match matches.value_of("x") {
        Some(s) => Some(process_u32(s).chain_err(|| "Inappropriate type of arguments in x")?),
        None => None,
    };

    let y = match matches.value_of("y") {
        Some(s) => Some(process_u32(s).chain_err(|| "Inappropriate type of arguments in y")?),
        None => None,
    };

    let width = match matches.value_of("width") {
        Some(s) => Some(process_u32(s).chain_err(|| "Inappropriate type of arguments in width")?),
        None => None,
    };

    let height = match matches.value_of("height") {
        Some(s) => Some(process_u32(s).chain_err(|| "Inappropriate type of arguments in height")?),
        None => None,
    };

    let input = matches.value_of("input");
    let output = matches.value_of("output");

    reader::read(input)
        .chain_err(|| "Problem with the input")?
        .save_to_png(eye, at, x, y, width, height, output)?;

    Ok(())
}

fn process(vec: Vec<&str>) -> Result<nalgebra::Point3<f32>> {
    Ok(nalgebra::Point3::new(
        vec[0].parse::<f32>()?,
        vec[1].parse::<f32>()?,
        vec[2].parse::<f32>()?,
    ))
}

fn process_u32(s: &str) -> Result<u32> {
    Ok(s.parse::<u32>()?)
}
