#[macro_use]
extern crate error_chain;
extern crate iswr;
use clap::{App, Arg};
use iswr::{errors::*, reader::read, renderer::Renderer, PlyDir};
use std::path::Path;

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
        .arg(
            Arg::with_name("background")
                .short("b")
                .long("background")
                .use_delimiter(true)
                .takes_value(true)
                .multiple(false)
                .help("Color of background"),
        )
        .arg(
            Arg::with_name("width")
                .short("w")
                .long("width")
                .use_delimiter(true)
                .takes_value(true)
                .multiple(false)
                .help("Position of at"),
        )
        .arg(
            Arg::with_name("height")
                .short("h")
                .long("height")
                .use_delimiter(true)
                .takes_value(true)
                .multiple(false)
                .help("Position of at"),
        )
        .get_matches();

    let input = matches.value_of("input");

    let eye = match matches.values_of("eye") {
        Some(vec) => Some(
            Some(vec.collect::<Vec<_>>())
                .filter(|vec| vec.len() == 3)
                .map(process_vec)
                .chain_err(|| "Inappropriate number of arguments in eye")?
                .chain_err(|| "Inappropriate type of arguments in eye, should be float number")?,
        ),
        None => None,
    };

    let at = match matches.values_of("at") {
        Some(vec) => Some(
            Some(vec.collect::<Vec<_>>())
                .filter(|vec| vec.len() == 3)
                .map(process_vec)
                .chain_err(|| "Inappropriate number of arguments in at, need 3 arguments")?
                .chain_err(|| "Inappropriate type of arguments in at, should be float number {}")?,
        ),
        None => None,
    };

    let background_color = match matches.values_of("background") {
        Some(vec) => Some(
            Some(vec.collect::<Vec<_>>())
                .filter(|vec| vec.len() == 3)
                .map(process_vec)
                .chain_err(|| "Inappropriate number of arguments in background, need 3 arguments")?
                .chain_err(|| {
                    "Inappropriate type of arguments in background, should be float number {}"
                })?,
        ),
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

    // let mut renderer = Renderer::new(None, width, height);
    // renderer.config_camera(eye, at);
    // renderer.config_background_color(background_color);

    let config_rederer_with_title = |r: &mut Renderer| {
        r.config_camera(eye, at);
        r.config_background_color(background_color);
    };

    match input {
        Some(path) => {
            let new_path = Path::new(&path);
            if new_path.is_file() {
                let ply = read(input)
                    .chain_err(|| format!("{}{}", "Problem with the input: ", input.unwrap()))?;

                let mut renderer = Renderer::new(ply.get_title(), width, height);
                config_rederer_with_title(&mut renderer);
                renderer.render_image(&ply.get_points());
            } else if new_path.is_dir() {
                let ply_dir = PlyDir::new(path);

                let mut renderer = Renderer::new(ply_dir.get_title(), width, height);
                config_rederer_with_title(&mut renderer);
                renderer
                    .render_video(ply_dir)
                    .chain_err(|| "Something went wrong")?;
            } else {
                eprintln!("No such file or dir {}", path)
            }
        }
        None => {
            let ply = read(input).chain_err(|| "Problem with the input")?;
            let mut renderer = Renderer::new(ply.get_title(), width, height);
            config_rederer_with_title(&mut renderer);
            renderer.render_image(ply.get_points_as_ref());
        }
    };

    Ok(())
}

fn process_vec(vec: Vec<&str>) -> Result<nalgebra::Point3<f32>> {
    Ok(nalgebra::Point3::new(
        vec[0].parse::<f32>()?,
        vec[1].parse::<f32>()?,
        vec[2].parse::<f32>()?,
    ))
}

fn process_u32(s: &str) -> Result<u32> {
    Ok(s.parse::<u32>()?)
}
