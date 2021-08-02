extern crate iswr;
use std::io::Error;
use clap::{App, Arg};
use iswr::{filter, transform, reader};

fn main() {
    // if let Err(e) = run() {
    //     println!("{}", e);
    // }

    run().unwrap();
}

fn run() -> Result<(), Error> {
    let matches = App::new("ply_fat")
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
            Arg::with_name("filter")
                .long("filter")
                .takes_value(true)
                .multiple(false)
                .help("Filter method"),
        )
        .arg(
            Arg::with_name("transform")
                .short("t")
                .long("transform")
                .takes_value(true)
                .multiple(false)
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
    let remain = matches
        .value_of("remain")
        .unwrap_or(transform::DEFAULT_KEY);
    let form = matches.value_of("form");
    let output = matches.value_of("output");
    let data = reader::read(input);

    let filter_methods = filter::get_collection();
    let transform_methods = transform::get_collection();

    // data.fat(
    //     filter_methods.get(filter).expect("Filter method not found"),
    //     transform_methods.get(transform).expect("transform method not found"),
    //     transform_methods.get(remain).expect("transform method for remain points not found"),
    // )
    // .write(form, output)

    println!{"Hasagi {}", filter};

    data.fat(
        filter_methods.get(filter).unwrap(),
        transform_methods.get(transform).expect("transform method not found"),
        transform_methods.get(remain).expect("transform method for remain points not found"),
    )
    .write(form, output)
}