extern crate iswr;
use iswr::Config;
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new_with_one_arg(&args).unwrap();

    if let Err(e) = run(config) {
        println!("Application error: {}", e);
        process::exit(1);
    }
}

fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let source = config.filename1;
    let data = iswr::materials::ply_file::PlyFile::new(&source)
        .unwrap()
        .read();
    let mut target = PathBuf::from(iswr::OUT_DIR.to_owned());
    target.push("binary");
    target.push(PathBuf::from(&source).file_name().unwrap());

    print!("Writing as binary to {:?}", target);

    iswr::materials::ply_file::PlyFile::create(target.to_str().unwrap())
        .unwrap()
        .writen_as_binary(data)?;

    Ok(())
}
