mod channel;
mod executor;
mod subcommands;

use std::sync::mpsc::Receiver;

use crate::{
    formats::{pointxyzrgba::PointXyzRgba, PointCloud},
    metrics::Metrics,
};

use self::{
    executor::Executor,
    subcommands::{Downsampler, MetricsCalculator, Read, Subcommand, ToPng, Write},
};

pub type SubcommandCreator = Box<dyn Fn(Vec<String>) -> Box<dyn Subcommand>>;

fn subcommand(s: &str) -> Option<SubcommandCreator> {
    match s {
        "write" => Some(Box::from(Write::from_args)),
        "to_png" => Some(Box::from(ToPng::from_args)),
        "read" => Some(Box::from(Read::from_args)),
        "metrics" => Some(Box::from(MetricsCalculator::from_args)),
        "downsample" => Some(Box::from(Downsampler::from_args)),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub enum PipelineMessage {
    PointCloud(PointCloud<PointXyzRgba>),
    Metrics(Metrics),
    End,
}

#[derive(Debug)]
pub enum Progress {
    Incr,
    Completed,
}
pub struct Pipeline;

impl Pipeline {
    pub fn execute() {
        let (mut executors, progresses) = Self::gather_pipeline_from_args();
        let mut handles = vec![];
        let mut names = vec![];
        let mut progress_recvs = vec![];
        let all_input_names: Vec<Vec<String>> = executors.iter().map(|e| e.input_names()).collect();

        for (idx, input_names) in all_input_names.iter().enumerate() {
            let mut inputs = vec![];
            for input_name in input_names {
                for executor in &mut executors {
                    if executor.output_name().eq(input_name) {
                        inputs.push(executor.output());
                    }
                }
            }
            executors[idx].set_inputs(inputs);
        }

        for (exec, progress) in executors.into_iter().zip(progresses) {
            names.push(exec.name());
            progress_recvs.push(progress);
            handles.push(exec.run());
        }

        let mut completed = 0;
        let mut progress = vec![0; progress_recvs.len()];
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
                    }
                }
            }
            println!("=======================");
            for i in 0..progress.len() {
                println!("{}: {}", names[i], progress[i])
            }
            println!("=======================");
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        for handle in handles {
            handle.join().expect("Failed to wait for thread");
        }
    }

    fn gather_pipeline_from_args() -> (Vec<Executor>, Vec<Receiver<Progress>>) {
        let args = std::env::args();
        let mut executors = vec![];
        let mut progresses = vec![];
        let mut command_creator: Option<SubcommandCreator> = None;
        let mut accumulated_args: Vec<String> = vec![];

        for arg in args.skip(1) {
            let is_command = subcommand(&arg);
            if is_command.is_some() {
                if let Some(creator) = command_creator.take() {
                    let forwarded_args = accumulated_args;
                    accumulated_args = vec![];
                    let (executor, progress) = Executor::create(forwarded_args, creator);
                    executors.push(executor);
                    progresses.push(progress);
                }
                command_creator = is_command;
            }
            accumulated_args.push(arg);
        }
        let creator = command_creator
            .take()
            .expect("Should have at least one command");

        let (executor, progress) = Executor::create(accumulated_args, creator);
        executors.push(executor);
        progresses.push(progress);
        (executors, progresses)
    }
}
