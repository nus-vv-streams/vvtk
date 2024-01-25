use std::{ffi::OsString, fs, io, path::{PathBuf, Path}, process::Command};

use clap::Parser;

use crate::pipeline::{channel::Channel, PipelineMessage};

use super::Subcommand;

#[derive(Parser)]
#[clap(
    about = "This commmand will run the extension subcommand"
)]
pub struct Args {
    //command name of the extension
    cmd_name: String,
    // If it is a binaries, where to find the binary paths, TODO, make it a vector of path later
    // default path for internal subcommand is ~/.cargo/
    binary_paths: String,
}
pub struct Extension {
    args: Args, 
}

impl Extension {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        Box::from(Extension {
            args: Args::parse_from(args),
        })
    }
}

impl Subcommand for Extension {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        //TODO: add more args for function arguments, try with the no command line argument version first
        println!("handle function from extension is executed, and here are the args");
        let testdir = PathBuf::from(&self.args.binary_paths);
        let mut paths = vec![testdir];
        execute_subcommand_executable(paths, &self.args.cmd_name, vec!["name_here"]);
        channel.send(PipelineMessage::End);
    }
}

fn find_subcommand_executable(paths:Vec<PathBuf>, cmd: &str) -> Option<PathBuf> {
    let command_name = format!("vv-{}{}", cmd, std::env::consts::EXE_SUFFIX);
    print!("command name: {}", command_name);
    paths.iter()
         .map(|dir| dir.join(&command_name))
         .find(|file| is_executable(file))
}

fn execute_subcommand_executable(paths:Vec<PathBuf>, cmd: &str, cmd_args:Vec<&str>) -> Result<(), &'static str> {
    let path = find_subcommand_executable(paths, cmd);
    let command = match path {
        Some(command) => command, 
        None => {
            // use println for now, need proper handling
            println!("The external command not found!");
            //TODO: fix this part
            return Err("Invalid comand");
        }
    };
    execute_external_subcommand(Some(&command), cmd_args)
}

// This function will execute subcommand that is stored as rust code
fn execute_rust_subcommand(cmd_path: Option<&PathBuf>, cmd_args:Vec<&str>) -> Result<(), &'static str> {
    execute_external_subcommand(None, cmd_args)
}

// execute either code or binaries 
fn execute_external_subcommand(cmd_path: Option<&PathBuf>, cmd_args:Vec<&str>) -> Result<(), &'static str> {
    // only implement external subcommand for now
    match cmd_path {
        //this is a test to pass a vector of args to an executable
        Some(cmd_path) => {
            println!("this is a valid external subcommand");
            Command::new(cmd_path)
            .args(cmd_args)
            .spawn()
            .expect("Failed to run the executable");
            return Ok(());
        },
        None => {
            println!("this is internal subcommand, not implemented yet");
            //TODO: implement someting here
            return Err("Internal subcommand not implemented yet");
        }
    }
}


// implement the logic for is_executable
// copied from cargo, and there is another version for window
// TODO: fix this part
fn is_executable<P: AsRef<Path>>(path: P) -> bool {
    use std::os::unix::prelude::*;
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

//TODO: need to design to pass the result to the next pipeline
