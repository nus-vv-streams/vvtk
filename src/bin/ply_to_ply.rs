#[macro_use]
extern crate error_chain;
extern crate vivotk;
use clap::{App, Arg};
use vivotk::{errors::*, io::reader, io::writer};

quick_main!(run);

fn run() -> Result<()> {
    let matches = App::new("ply_to_ply")
        .about("Write data to ply file in ascii or binary form")
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .takes_value(true)
                .multiple_occurrences(false)
                .help("File directory for input"),
        )
        .arg(
            Arg::new("form")
                .short('f')
                .long("form")
                .takes_value(true)
                .multiple_occurrences(false)
                .help("Form of output (ascii/binary)"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .takes_value(true)
                .multiple_occurrences(false)
                .help("File directory for output"),
        )
        .get_matches();

    let input = matches.value_of("input");
    let form = matches.value_of("form");
    let output = matches.value_of("output");

    let ply = reader::read(input).chain_err(|| "Problem with the input")?;

    // ply.get_points()
    //     .write(form, output)
    //     .chain_err(|| "Problem with the output")?;

    writer::write(ply.get_points(), form, output).chain_err(|| "Problem with the output")?;

    Ok(())
}
