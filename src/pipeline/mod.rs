mod executor;
pub mod subcommands;


use std::sync::mpsc::Receiver;

use crate::formats::{pointxyzrgba::PointXyzRgba, PointCloud};

use self::{
    executor::Executor,
    subcommands::{Metrics, Read, Subcommand, ToPng, Write, Convert},
};

use clearscreen;

pub type SubcommandCreator = Box<dyn Fn(Vec<String>) -> Box<dyn Subcommand>>;

fn subcommand(s: &str) -> Option<SubcommandCreator> {
    match s {
        "write" => Some(Box::from(Write::from_args)),
        "to_png" => Some(Box::from(ToPng::from_args)),
        "read" => Some(Box::from(Read::from_args)),
        "metrics" => Some(Box::from(Metrics::from_args)),
        "convert" => Some(Box::from(Convert::from_args)),
        _ => None,
    }
}

#[derive(Debug)]
pub enum PipelineMessage {
    PointCloud(PointCloud<PointXyzRgba>),
    End,
}

#[derive(Debug)]
pub enum Progress {
    Incr,
    Length(usize),
    Completed,
}
pub struct Pipeline;

impl Pipeline {
    pub fn execute() {
        let pipeline = Self::gather_pipeline_from_args();
        println!("Executing pipeline");

        let mut handles = vec![];
        let mut names = vec![];
        let mut progress_recvs = vec![];
        for (exec, progress) in pipeline {
            names.push(exec.name());
            progress_recvs.push(progress);
            handles.push(exec.run());
        }

        let mut completed = 0;
        let mut progress = vec![0; progress_recvs.len()];
        let mut length = 0;
        println!("progress_recvs.len(): {}", progress_recvs.len());
        while completed < progress_recvs.len() {
            for (idx, recv) in progress_recvs.iter().enumerate() {
                while let Ok(prog) = recv.try_recv() {
                    match prog {
                        Progress::Incr => {
                            progress[idx] += 1;
                        }
                        Progress::Completed => {
                            completed += 1;
                        }
                        Progress::Length(l) => {
                            length = l;
                        }
                    }
                }
            }
            println!("Progress completed: {:?}", completed);
            // clearscreen::clear().expect("Failed to clear screen");
            for i in 0..progress.len() {
                println!("{}: {} / {}", names[i], progress[i], length)
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        for handle in handles {
            handle.join().expect("Failed to wait for thread");
        }
    }

    fn gather_pipeline_from_args() -> Vec<(Executor, Receiver<Progress>)> {
        let args = std::env::args();
        let mut pipeline: Vec<(Executor, Receiver<Progress>)> = vec![];
        let mut command_creator: Option<SubcommandCreator> = None;
        let mut accumulated_args: Vec<String> = vec![];
        let mut prev_recv: Option<Receiver<PipelineMessage>> = None;

        for arg in args.skip(1) {
            let is_command = subcommand(&arg);
            if is_command.is_some() {
                if let Some(creator) = command_creator.take() {
                    let forwarded_args = accumulated_args;
                    accumulated_args = vec![];
                    let (mut executor, recv, progress) = Executor::create(forwarded_args, creator);
                    if let Some(recv) = prev_recv.take() {
                        executor.set_input(recv);
                    }
                    prev_recv = Some(recv);
                    pipeline.push((executor, progress));
                }
                command_creator = is_command;
            }
            accumulated_args.push(arg);
        }
        let creator = command_creator
            .take()
            .expect("Should have at least one command");

        let (mut executor, _, progress) = Executor::create(accumulated_args, creator);
        if let Some(recv) = prev_recv.take() {
            executor.set_input(recv);
        }
        pipeline.push((executor, progress));

        pipeline
    }
}
