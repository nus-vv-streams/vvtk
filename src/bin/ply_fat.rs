#[macro_use]
extern crate error_chain;
extern crate iswr;
use clap::{App, Arg};
use iswr::{errors::*, filter, reader, transform};

quick_main!(run);

fn run() -> Result<()> {
    let matches = App::new("ply_fat")
        .about("Filter and Transform points")
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .takes_value(true)
                .multiple(false)
                .help("File directory for data"),
        )
        .arg(
            Arg::with_name("filter")
                .long("filter")
                .takes_value(true)
                .multiple(false)
                .required(true)
                .help("Filter method"),
        )
        .arg(
            Arg::with_name("transform")
                .short("t")
                .long("transform")
                .takes_value(true)
                .multiple(false)
                .required(true)
                .help("Transform method"),
        )
        .arg(
            Arg::with_name("remain")
                .short("r")
                .long("remain")
                .takes_value(true)
                .multiple(false)
                .help("Transform method"),
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
    let filter = matches.value_of("filter").unwrap_or(filter::DEFAULT_KEY);
    let transform = matches
        .value_of("transform")
        .unwrap_or(transform::DEFAULT_KEY);
    let remain = matches.value_of("remain").unwrap_or(transform::DEFAULT_KEY);
    let form = matches.value_of("form");
    let output = matches.value_of("output");
    let data = reader::read(input).chain_err(|| "Problem with the input")?;
    let filter_methods = filter::get_collection();
    let transform_methods = transform::get_collection();

    data.fat(
        filter_methods.get(filter),
        transform_methods.get(transform),
        transform_methods.get(remain),
    )
    .chain_err(|| "Problem with the Filter & Transform methods")?
    .write(form, output)
    .chain_err(|| "Problem with the output")?;

    Ok(())
}
