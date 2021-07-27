extern crate iswr;
use iswr::Config;
use std::env;
use std::error::Error;
use std::path::Path;
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
    let new_path = Path::new(&source);
    if new_path.is_file() {
        iswr::materials::ply_file::PlyFile::new(&source)
            .unwrap()
            .render();
    } else if new_path.is_dir() {
        iswr::materials::ply_dir::PlyDir::new(&source).play();
    }

    Ok(())
}
