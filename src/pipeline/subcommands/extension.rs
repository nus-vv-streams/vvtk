use std::{fs, io::{self, Write}, path::{Path, PathBuf}, process::{Command, Stdio}, env};

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::{formats::{pointxyzrgba::PointXyzRgba, PointCloud}, pipeline::{channel::Channel, PipelineMessage}};

use super::Subcommand;

#[derive(Parser)]
#[clap(
    about = "vv extend is used for custom subcommands."
)]
pub struct Args {
    // Command name of the extension
    cmd_name: String,
    // Arguments that needs to pass in to the binary executable, value separate by comma
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
        // default cargo home, if not go $PATH
        // Search through path directory
        let key = "CARGO_HOME";
        match env::var_os(key) {
            Some(val) => println!("{key}: {val:?}"),
            None => {
                println!("{key} is not defined in the environment.");
                return;
             }
        }
        let testdir = PathBuf::from(env::var_os(key).unwrap()).join("bin"); 
        println!("testdir is {:?}", testdir);
        let paths: Vec<PathBuf> = vec![testdir];
        println!("path is {:?}", paths);
        let mut input_pc: Option<PointCloud<PointXyzRgba>> = None;
        let mut should_execute_subcommand = false;
        let mut pc_index: Option<u32> = None;
        for message in messages {
            // Didn't handle PointCloud<PointXyzRgbaNormal>
            match &message {
                PipelineMessage::SubcommandMessage(subcommand_object,index) => {
                    input_pc = Some(*(subcommand_object.content).clone());
                    pc_index = Some(index.clone());
                    should_execute_subcommand = true;
            }
            PipelineMessage::IndexedPointCloud(pc, index) => {
                input_pc = Some(pc.clone());
                pc_index = Some(index.clone());
                should_execute_subcommand = true;
            }
            PipelineMessage::End => {
                println!("vv extend received pipeline end");
                channel.send(PipelineMessage::End);
            }
            _ => {
                channel.send(message);
            }
            }
        }
        println!("the point cloud received by vv extend is {:?}", input_pc);
        println!("should execute subcommad is {:?}", should_execute_subcommand);
        if should_execute_subcommand {
            //let result = execute_subcommand_executable(paths, &self.args.cmd_name, &self.args.xargs, input_pc);
            //println!("the result of execution is {:?}", result);
            let result = execute_subcommand_executable(paths, &self.args.cmd_name, &self.args.xargs, input_pc);
            match result {
                Ok(child_deserialized_output) => {
                    // send the message to the channel
                    println!("vv extend sent pointcloud");
                    channel.send(PipelineMessage::SubcommandMessage(child_deserialized_output, pc_index.unwrap()));
                }
                Err(e) => {
                    println!("vv extend sent pipeline end");
                    eprintln!("Error: {}", e);
                    channel.send(PipelineMessage::End);
                }
            }
            /* 
            if let Ok(child_deserialized_output) = 
            // None because right not extend is executed as the first command, will change later
                execute_subcommand_executable(paths, &self.args.cmd_name, &self.args.xargs, input_pc) {
                // send the message to the channel
                println!("vv extend sent pointcloud");
                channel.send(PipelineMessage::SubcommandMessage(child_deserialized_output, pc_index.unwrap()));
            }
            else {
                println!("vv extend sent pipeline end");
                channel.send(PipelineMessage::End);
            }
            */
        }
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
    println!("execute_subcommand_executable is called");
    println!("path is {:?}", paths);
    println!("cmd is {:?}", cmd);
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
    // vv extend expects to receive a pointCloud, and also output a point cloud to the pipeline
    println!("execute_external_subcommand is called");
    println!("cmd path is {:?}", cmd_path);
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
            child_deserialized_output = Some(serde_json::from_str(&child_stdout).unwrap());
            match child_deserialized_output {
                Some(child_deserialized_output) => Ok(child_deserialized_output),
                None => Err("Failed to get deserialized output of the child process"),
            }
        },
        None => {
            Err("Command path not found")
        }
    }
}


// is_executable implementation referred Rust cargo src/bin/cargo/main.rs
// https://github.com/rust-lang/cargo/blob/master/src/bin/cargo/main.rs
#[cfg(unix)]
fn is_executable<P: AsRef<Path>>(path: P) -> bool {
    use std::os::unix::prelude::*;
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}
#[cfg(windows)]
fn is_executable<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().is_file()
}

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
