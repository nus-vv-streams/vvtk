use std::{fs, io::{self, Write}, path::{Path, PathBuf}, process::{Command, Stdio}};

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::{formats::pointxyzrgba::PointXyzRgba, pipeline::{channel::Channel, PipelineMessage}};

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
    //TODO: use clap for this to parse a new vec   
    #[clap(short, long, value_parser, num_args = 1.., value_delimiter = ',')]
    xargs: Vec<String>,
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
    // This will be called by the executor to execute this particular subcommand
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        // TODO: handle the input of the pipeline
        let testdir = PathBuf::from(&self.args.binary_paths);
        let mut paths: Vec<PathBuf> = vec![testdir];
        //TODO: pick String or &str
        //let mut cmd_args:&Vec<String> = &vec!["".to_string()];
        execute_subcommand_executable(paths, &self.args.cmd_name, &self.args.xargs);
        channel.send(PipelineMessage::End);
    }
}

fn find_subcommand_executable(paths:Vec<PathBuf>, cmd: &str) -> Option<PathBuf> {
    let command_name = format!("vv-{}{}", cmd, std::env::consts::EXE_SUFFIX);
    paths.iter()
         .map(|dir| dir.join(&command_name))
         .find(|file| is_executable(file))
}

fn execute_subcommand_executable(paths:Vec<PathBuf>, cmd: &str, cmd_args:&Vec<String>) -> Result<(), &'static str> {
    let path = find_subcommand_executable(paths, cmd);
    let command = match path {
        Some(command) => command, 
        None => {
            // use println for now, need proper handling
            //println!("The external command not found!");
            //TODO: fix this part
            return Err("Invalid comand");
        }
    };
    execute_external_subcommand(Some(&command), cmd_args)
}

// This function will execute subcommand that is stored as rust code
fn execute_rust_subcommand(cmd_path: Option<&PathBuf>, cmd_args:&Vec<String>) -> Result<(), &'static str> {
    execute_external_subcommand(None, cmd_args)
}

// execute either external code or binaries 
fn execute_external_subcommand(cmd_path: Option<&PathBuf>, cmd_args:&Vec<String>) -> Result<(), &'static str> {
    // only implement external subcommand for now
    /* 
    /* sucessful test for string, prepare for the subcommand test, add a formal test later */
    let input: SubcommandObject<String> = SubcommandObject::new(String::from("testing input string"));
    let serialized = serde_json::to_string(&input).unwrap();
    println!("serialized data = {}", serialized);
    let deserialized: SubcommandObject<String> = serde_json::from_str(&serialized).unwrap();
    println!("deserialized data = {}", deserialized.content);
    */
    let input: SubcommandObject<PointXyzRgba> = SubcommandObject::new(PointXyzRgba {
        x: 1.0,
        y: 2.0,
        z: 3.0,
        r: 4,
        g: 5,
        b: 6,
        a: 7,
    });
    let serialized = serde_json::to_string(&input).unwrap();
    println!("serialized data = {}", serialized);
    /* 
    let deserialized: SubcommandObject<PointXyzRgba> = serde_json::from_str(&serialized).unwrap();
    println!("deserialized data = {:?}", deserialized.content);
     */
    match cmd_path {
        //this is a test to pass a vector of args to an executable
        Some(cmd_path) => {
            let mut child = Command::new(cmd_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .args(cmd_args)
            .spawn()
            .expect("Failed to spawn child process");
            // TODO: If there is input stream, expect input, do it after the args, refer to metrics
            //TODO: exit status
            // show the output and error message
            let mut stdin = child.stdin.take().expect("Failed to open stdin");
            // pass the string as the stdin of the child process
            std::thread::spawn(move || {
                stdin.write_all(serialized.as_bytes()).expect("Failed to write to stdin");
            });
            let output = child.wait_with_output().expect("Failed to read stdout");
            io::stderr().write_all(&output.stderr).unwrap();    
            io::stdout().write_all(&output.stdout).unwrap();
            // TODO: transfer the process output to the pipeline
            // TODO: create another executable that can print message to test on this
            return Ok(());
        },
        None => {
            //println!("this is internal subcommand, not implemented yet");
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

//TODO: move this somewhere else when tidy up
#[derive(Debug, Serialize, Deserialize)]
pub struct SubcommandObject<T:Clone + Serialize> {
    content: Box<T>,
}

impl<T:Clone + Serialize> SubcommandObject<T> {
    pub fn new(content: T) -> Self {
        Self {
            content: Box::new(content),
        }
    }
}

impl<T:Clone + Serialize> Clone for SubcommandObject<T> {
    fn clone(&self) -> Self {
        Self {
            content: self.content.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{io::{self, Write as _}, process::{Command, Stdio}};

    use crate::{formats::pointxyzrgba::PointXyzRgba, pipeline::subcommands::extension::SubcommandObject};

    use super::execute_external_subcommand;

    #[test]
    fn pass_pointcloud_to_executable_child_process() {
        // This tests contains subset of code in execute_external_subcommand function, need clean up later
        // This test deserialize point cloud and pass it to executable child process, then child process will deserialize and print it
        // No cli arg passing for this test
        // TODO: clean up the binaries and improve this test
    let input: SubcommandObject<PointXyzRgba> = SubcommandObject::new(PointXyzRgba {
        x: 1.0,
        y: 2.0,
        z: 3.0,
        r: 4,
        g: 5,
        b: 6,
        a: 7,
    });
    let serialized = serde_json::to_string(&input).unwrap();
    //TODO: make sure this binaries is imported to github before pr
    // vv-test-pipe-pc-only will take in serialized point cloud, deserialized it, and print the struct
    let mut child = Command::new("../test_binaries/vv-test-pipe-pc-only")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()
    .expect("Failed to spawn child process");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");

    // Pipe the serialized point cloud to child process
    std::thread::spawn(move || {
        stdin.write_all(serialized.as_bytes()).expect("Failed to write to stdin");
    });
    let output = child.wait_with_output().expect("Failed to read stdout");
    // Take out the point cloud from SubcommandObject
    let subcommand_content = input.content;
    // add \n to the right because prinln! is used in the child executable
    println!("the point cloud created by parent process is {subcommand_content:?}");
    print!("the deserialized point cloud of child process is {}", String::from_utf8_lossy(&output.stdout));
    assert_eq!(String::from_utf8_lossy(&output.stdout), format!("{subcommand_content:?}\n"));
    }
}
