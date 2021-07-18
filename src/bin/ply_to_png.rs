extern crate iswr;
use std::env;
use std::process;
use iswr::Config;
use std::error::Error;
use std::path::{ PathBuf };

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap();

    if let Err(e) = run(config) {
        println!("Application error: {}", e);
        process::exit(1);
    }
}

fn run(config: Config) -> Result<(), Box<dyn Error>> {
    if config.filename2 == iswr::NOTHING {
        iswr::materials::ply_file::PlyFile::new(&config.filename1).unwrap().take_sreenshoot();
        Ok(())
    } else {
        let mut target = PathBuf::from(iswr::OUT_DIR.to_owned());
        target.push("png");
        target.push(PathBuf::from(&config.filename2).file_name().unwrap());
    
        iswr::materials::ply_file::PlyFile::new(&config.filename1).unwrap().take_sreenshoot_to_path(target.to_str().unwrap());
        Ok(())
    }
    
}