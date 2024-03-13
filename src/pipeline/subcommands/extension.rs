use std::{fs, io::{self, Write}, path::{Path, PathBuf}, process::{Command, Stdio}};

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::{formats::{pointxyzrgba::PointXyzRgba, PointCloud}, pipeline::{channel::Channel, PipelineMessage}};

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
        println!("this handle is invoked");
        let testdir = PathBuf::from(&self.args.binary_paths);
        println!("testdir is {:?}", testdir);
        let paths: Vec<PathBuf> = vec![testdir];
        println!("path is {:?}", paths);
        let mut input_pc: Option<PointCloud<PointXyzRgba>> = None;
        for message in messages {
            // Current assumption of vv extend is it only takes in PointCloud and output a point cloud
            // Didn't handle PointCloud<PointXyzRgbaNormal>  for now
            match &message {
                PipelineMessage::SubcommandMessage(subcommand_object,) => {
                    input_pc = Some(*(subcommand_object.content).clone());
            }
            PipelineMessage::IndexedPointCloud(pc, _) => {
                input_pc = Some(pc.clone());
            }
            PipelineMessage::End => {
                channel.send(PipelineMessage::End);
            }
            _ => {}
            }
        }
        println!("the point cloud received by vv extend is {:?}", input_pc);
        if let Ok(child_deserialized_output) = 
        // None because right not extend is executed as the first command, will change later
            execute_subcommand_executable(paths, &self.args.cmd_name, &self.args.xargs, input_pc) {
            // send the message to the channel
            // TODO: remove the bool here completely
            channel.send(PipelineMessage::SubcommandMessage(child_deserialized_output));
            println!("The command is sent!");
            // //TODO: implement a function to convert from string to PointXyzRgba for SubcommandObject
        }
        else {
            println!("pipeline message end is executed!");
            channel.send(PipelineMessage::End);
        }
        println!("handle ends here");
    }
}
// Find the executable which has the name "vv-(cmd)" in all the paths listed in paths
fn find_subcommand_executable(paths:Vec<PathBuf>, cmd: &str) -> Option<PathBuf> {
    let command_name = format!("vv-{}{}", cmd, std::env::consts::EXE_SUFFIX);
    paths.iter()
         .map(|dir| dir.join(&command_name))
         .find(|file| is_executable(file))
}

// Execute the subcommand that is in executable form
fn execute_subcommand_executable(paths:Vec<PathBuf>, cmd: &str, cmd_args:&Vec<String>, input_pc: Option<PointCloud<PointXyzRgba>>) -> Result<SubcommandObject<PointCloud<PointXyzRgba>>, &'static str> {
    let path = find_subcommand_executable(paths, cmd);
    let command = match path {
        Some(command) => command, 
        None => {
            return Err("The executable is not found");
        }
    };
    execute_external_subcommand(Some(&command), cmd_args, input_pc)
}

// execute external code or binaries 
fn execute_external_subcommand(cmd_path: Option<&PathBuf>, cmd_args:&Vec<String>, input_pc: Option<PointCloud<PointXyzRgba>>) -> Result<SubcommandObject<PointCloud<PointXyzRgba>>, &'static str> {
    println!("input pc is {:?}", input_pc);
    // Should receive a pointCloud, and also output a point cloud to the pipeline
    let input;
    match input_pc {
        Some(input_pc) => {
            input = SubcommandObject::new(input_pc);
        }
        None => {
            return Err("No input point cloud for vv extend");
        }
    }
    let serialized = serde_json::to_string(&input).unwrap();
    match cmd_path {
        Some(cmd_path) => {
            let mut child = Command::new(cmd_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .args(cmd_args)
            .spawn()
            .expect("Failed to spawn child process");
            let mut stdin = child.stdin.take().expect("Failed to open stdin");
            // pass the string as the stdin of the child process
            std::thread::spawn(move || {            
                stdin.write_all(serialized.as_bytes()).expect("Failed to write to stdin");
            });
            let output = child.wait_with_output().expect("Failed to read stdout");
            //print exit code of the child process
            match &output.status.code() {
                Some(code) => println!("Subprocess exited with status code: {}", code),
                None => println!("Process terminated by signal"),
            }
            //print error and stdout from child process
            io::stderr().write_all(&output.stderr).unwrap();    
            io::stdout().write_all(&output.stdout).unwrap();
            //pass the SubcommandObject<PointCloud> back to the pipeline
            let child_stdout:String = String::from_utf8(output.stdout.clone()).unwrap();
            let child_deserialized_output: Option<SubcommandObject<PointCloud<PointXyzRgba>>>;
            // TODO: clean up this part
            /* 
            let _ = serde_json::from_str::<SubcommandObject<PointCloud<PointXyzRgba>>>(&child_stdout) {
                Ok(value) => Ok(value), 
                Err(error) => {
                    println!("error is {:?}", error);
                    Err("Failed to get deserialized output of the child process")
                } 
            }
            */
            child_deserialized_output = Some(serde_json::from_str(&child_stdout).unwrap());
            match child_deserialized_output {
                Some(child_deserialized_output) => Ok(child_deserialized_output),
                None => Err("Failed to get deserialized output of the child process"),
            }
        },
        None => {
            //println!("this is internal subcommand, not implemented yet");
            //TODO: implement someting here
            Err("Internal subcommand not implemented yet")
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

    pub fn get_content(&self) -> &T {
        &self.content
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
