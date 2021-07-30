extern crate iswr;
use clap::{App, Arg};
use iswr::methods::{filter, transform};

fn main() {
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
                .short("f")
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
            Arg::with_name("transform_remain")
                .short("r")
                .long("transform_remain")
                .takes_value(true)
                .multiple(false)
                .help("Transform method"),
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
    let data = iswr::tool::reader::read(input);

    let filter = matches.value_of("filter").unwrap_or(filter::DEFAULT_KEY);
    let transform = matches
        .value_of("transform")
        .unwrap_or(transform::DEFAULT_KEY);
    let transform_remain = matches
        .value_of("transform_remain")
        .unwrap_or(transform::DEFAULT_KEY);

    let filter_methods = filter::get_collection();
    let transform_methods = transform::get_collection();

    data.fat(
        filter_methods.get(filter).unwrap(),
        transform_methods.get(transform).unwrap(),
        transform_methods.get(transform_remain).unwrap(),
    )
    .render();
}
