mod executor;
mod subcommands;

use std::{sync::mpsc::Receiver, thread::JoinHandle};

use crate::formats::{pointxyzrgba::PointXyzRgba, PointCloud};

use self::{
    executor::Executor,
    subcommands::{Play, Read, Subcommand, ToPng, Write},
};

pub type SubcommandCreator = Box<dyn Fn(Vec<String>) -> Box<dyn Subcommand>>;

fn subcommand(s: &str) -> Option<SubcommandCreator> {
    match s {
        "write" => Some(Box::from(Write::from_args)),
        "play" => Some(Box::from(Play::from_args)),
        "to_png" => Some(Box::from(ToPng::from_args)),
        "read" => Some(Box::from(Read::from_args)),
        _ => None,
    }
}

pub enum PipelineMessage {
    PointCloud(PointCloud<PointXyzRgba>),
    End,
}
pub struct Pipeline;

impl Pipeline {
    pub fn execute() {
        let pipeline = Self::gather_pipeline_from_args();

        let handles: Vec<JoinHandle<()>> = pipeline.into_iter().map(|exec| exec.run()).collect();
        for handle in handles {
            handle.join().expect("Failed to wait for thread");
        }
    }

    fn gather_pipeline_from_args() -> Vec<Executor> {
        let args = std::env::args();
        let mut pipeline: Vec<Executor> = vec![];
        let mut command_creator: Option<SubcommandCreator> = None;
        let mut accumulated_args: Vec<String> = vec![];
        let mut prev_recv: Option<Receiver<PipelineMessage>> = None;

        for arg in args.skip(1) {
            let is_command = subcommand(&arg);
            if is_command.is_some() {
                if let Some(creator) = command_creator.take() {
                    let forwarded_args = accumulated_args;
                    accumulated_args = vec![];
                    let (mut executor, recv) = Executor::create(forwarded_args, creator);
                    if let Some(recv) = prev_recv.take() {
                        executor.set_input(recv);
                    }
                    prev_recv = Some(recv);
                    pipeline.push(executor);
                }
                command_creator = is_command;
            }
            accumulated_args.push(arg);
        }

        if let Some(creator) = command_creator.take() {
            let (mut executor, _) = Executor::create(accumulated_args, creator);
            if let Some(recv) = prev_recv.take() {
                executor.set_input(recv);
            }
            pipeline.push(executor);
        }
        pipeline
    }
}
