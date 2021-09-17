#[macro_use]
extern crate error_chain;
extern crate iswr;
use clap::{App, Arg};
use iswr::{errors::*, reader};

quick_main!(run);

fn run() -> Result<()> {
    let matches = App::new("ply_to_ply")
        .about("Write data to ply file in ascii or binary form")
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .takes_value(true)
                .multiple(false)
                .help("File directory for input"),
        )
        .arg(
            Arg::with_name("form")
                .short("f")
                .long("form")
                .takes_value(true)
                .multiple(false)
                .help("Form of output (ascii/binary)"),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .takes_value(true)
                .multiple(false)
                .help("File directory for output"),
        )
        .get_matches();

    let input = matches.value_of("input");
    let form = matches.value_of("form");
    let output = matches.value_of("output");

    let ply = reader::read(input).chain_err(|| "Problem with the input")?;

    ply.get_points()
        .write(form, output)
        .chain_err(|| "Problem with the output")?;

    Ok(())
}
