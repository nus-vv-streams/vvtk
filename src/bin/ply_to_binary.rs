extern crate iswr;
use clap::{App, Arg};
use std::path::PathBuf;

fn main() {
    let matches = App::new("ply_to_binary")
        .about("Write data to ply file in binary form")
        .arg(
            Arg::with_name("source")
                .short("s")
                .long("source")
                .takes_value(true)
                .multiple(false)
                .help("File directory for data"),
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
    let source = matches.value_of("source").unwrap();
    let data = iswr::materials::ply_file::PlyFile::new(&source)
        .unwrap()
        .read();

    let mut defalt_output = PathBuf::from(iswr::OUT_DIR.to_owned());
    defalt_output.push("binary");
    defalt_output.push(PathBuf::from(&source).file_name().unwrap());

    let output = matches
        .value_of("output")
        .unwrap_or(defalt_output.to_str().unwrap());

    iswr::materials::ply_file::PlyFile::create(output)
        .unwrap()
        .writen_as_binary(data)
        .unwrap();

    print!("Writing as binary to {:?}", output);
}
