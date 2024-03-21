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
        // Search through cargo_directory
        let key = "CARGO_HOME";
        match env::var_os(key) {
            Some(val) => println!("{key}: {val:?}"),
            None => {
                println!("{key} is not defined in the environment.");
                return;
             }
        }
        let testdir = PathBuf::from(env::var_os(key).unwrap()).join("bin"); 
        let paths: Vec<PathBuf> = vec![testdir];
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
        if should_execute_subcommand {
            let result = execute_subcommand_executable(paths, &self.args.cmd_name, &self.args.xargs, input_pc);
            match result {
                Ok(child_deserialized_output) => {
                    channel.send(PipelineMessage::SubcommandMessage(child_deserialized_output, pc_index.unwrap()));
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    channel.send(PipelineMessage::End);
                }
            }
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
