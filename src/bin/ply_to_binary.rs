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
            Arg::with_name("target")
                .short("t")
                .long("target")
                .takes_value(true)
                .multiple(false)
                .help("File directory for target"),
        )
        .get_matches();
    let source = matches.value_of("source").unwrap();
    let data = iswr::materials::ply_file::PlyFile::new(&source)
        .unwrap()
        .read();

    let mut defalt_target = PathBuf::from(iswr::OUT_DIR.to_owned());
    defalt_target.push("binary");
    defalt_target.push(PathBuf::from(&source).file_name().unwrap());

    let target = matches
        .value_of("target")
        .unwrap_or(defalt_target.to_str().unwrap());

    iswr::materials::ply_file::PlyFile::create(target)
        .unwrap()
        .writen_as_binary(data)
        .unwrap();

    print!("Writing as binary to {:?}", target);
}
