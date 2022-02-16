#[macro_use]
extern crate error_chain;
extern crate vivotk;
use clap::{App, Arg};
use vivotk::{errors::*, io::reader, io::writer, seq::fat, seq::filter, seq::map};

quick_main!(run);

fn run() -> Result<()> {
    let matches = App::new("ply_fat")
        .about("Filter and Transform points")
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .takes_value(true)
                .multiple_occurrences(false)
                .help("File directory for data"),
        )
        .arg(
            Arg::new("filter")
                .long("filter")
                .takes_value(true)
                .multiple_occurrences(false)
                .required(true)
                .help("Filter method"),
        )
        .arg(
            Arg::new("transform")
                .short('t')
                .long("transform")
                .takes_value(true)
                .multiple_occurrences(false)
                .required(true)
                .help("Transform method"),
        )
        .arg(
            Arg::new("remain")
                .short('r')
                .long("remain")
                .takes_value(true)
                .multiple_occurrences(false)
                .help("Transform method"),
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
    let filter = matches.value_of("filter").unwrap_or(filter::DEFAULT_KEY);
    let transform = matches.value_of("transform").unwrap_or(map::DEFAULT_KEY);
    let remain = matches.value_of("remain").unwrap_or(map::DEFAULT_KEY);
    let form = matches.value_of("form");
    let output = matches.value_of("output");
    let ply = reader::read(input).chain_err(|| "Problem with the input")?;
    let data = ply.get_points();
    let filter_methods = filter::get_collection();
    let transform_methods = map::get_collection();

    let output_points = fat::fat(
        &data,
        filter_methods.get(filter),
        transform_methods.get(transform),
        transform_methods.get(remain),
    )
    .chain_err(|| "Problem with the Filter & Transform methods")?;

    writer::write(output_points, form, output).chain_err(|| "Problem with the output")?;

    Ok(())
}
