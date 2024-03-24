mod channel;
mod executor;
pub mod subcommands;
use clap::Parser;
use crossbeam_channel::Receiver;

use crate::{
    formats::{pointxyzrgba::PointXyzRgba, pointxyzrgbanormal::PointXyzRgbaNormal, PointCloud},
    metrics::Metrics,
};

use self::{
    executor::Executor,
    executor::ExecutorBuilder,
    subcommands::{
        convert, dash, downsample, info, metrics, normal_estimation, read, render, upsample, write, extension,
        Convert, Dash, Downsampler, Info, MetricsCalculator, NormalEstimation, Read, Render,
        Subcommand, Upsampler, Write, Extension,
    },
    subcommands::extension::SubcommandObject,
};

pub type SubcommandCreator = Box<dyn Fn(Vec<String>) -> Box<dyn Subcommand>>;

fn subcommand(s: &str) -> Option<SubcommandCreator> {
    match s {
        "write" => Some(Box::from(Write::from_args)),
        "render" => Some(Box::from(Render::from_args)),
        "read" => Some(Box::from(Read::from_args)),
        "metrics" => Some(Box::from(MetricsCalculator::from_args)),
        "downsample" => Some(Box::from(Downsampler::from_args)),
        "upsample" => Some(Box::from(Upsampler::from_args)),
        "convert" => Some(Box::from(Convert::from_args)),
        "normal" => Some(Box::from(NormalEstimation::from_args)),
        // "play" => Some(Box::from(Play::from_args)),
        "dash" => Some(Box::from(Dash::from_args)),
        "info" => Some(Box::from(Info::from_args)),
        "extend" => Some(Box::from(Extension::from_args)),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub enum PipelineMessage {
    IndexedPointCloud(PointCloud<PointXyzRgba>, u32),
    IndexedPointCloudNormal(PointCloud<PointXyzRgbaNormal>, u32),
    // PointCloud(PointCloud<PointXyzRgba>),
    Metrics(Metrics),
    // Pipeline for external subcommand, contains the object that subcommand wants to past
    // TODO: Add in i to match with IndexedPointCloud
    SubcommandMessage(SubcommandObject<PointCloud<PointXyzRgba>>, u32),
    End,
    DummyForIncrement,
}

#[derive(Debug)]
pub enum Progress {
    Incr,
    Completed,
}
pub struct Pipeline;

impl Pipeline {
    pub fn execute() {
        let (mut executors, progresses) = match Self::gather_pipeline_from_args() {
            Ok((executors, progresses)) => (executors, progresses),
            Err(err) => {
                println!("Error: {}", err);
                println!("Use --help for more information");
                return;
            }
        };

        let mut handles = vec![];
        let mut names = vec![];
        let mut progress_recvs = vec![];
        let all_input_names: Vec<Vec<String>> = executors.iter().map(|e| e.input_names()).collect();

        // !! set named input outputs
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

        // println!("progress_recvs.len(): {}", progress_recvs.len());
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
            /*
            println!("=======================");
            for i in 0..progress.len() {
                println!("{}: {}", names[i], progress[i])
            }
            println!("=======================");
            */
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        for handle in handles {
            handle.join().expect("Failed to wait for thread");
        }
    }

    // !! collect all the arguments from terminal and create the pipeline
    fn gather_pipeline_from_args() -> Result<(Vec<Executor>, Vec<Receiver<Progress>>), String> {
        let args: Vec<String> = std::env::args().collect();
        let mut executors = vec![];
        let mut progresses = vec![];
        let mut command_creator: Option<SubcommandCreator> = None;
        let mut accumulated_args: Vec<String> = vec![];

        let mut executor_builder = ExecutorBuilder::new();
        // !! check argument length
        if args.len() < 2 {
            display_main_help_msg();
            return Err("Expected at least one argument".to_string());
        }

        if args[1] == "--help" || args[1] == "-h" || args[1] == "help" {
            display_main_help_msg();
        }

        // !! check the second argument, which is the name of the subcommand, we want at least one subcommand
        if !Self::if_at_least_one_command(&args[1]) {
            return Err(format!(
                "Expected at least one valid command on the first arg, got {}",
                args[1]
            ));
        }

        // !! skip the first argument, which is the name of the program
        for arg in args.iter().skip(1) {
            let is_command = subcommand(arg);
            // If the subcommand is valid
            if is_command.is_some() {
                if let Some(creator) = command_creator.take()
                // !! the first take is always None
                {
                    // !! enters here when there are at least two subcommands
                    let forwarded_args = accumulated_args;
                    accumulated_args = vec![];
                    let (executor, progress) = executor_builder.create(forwarded_args, creator)?;
                    executors.push(executor);
                    progresses.push(progress);
                }
                command_creator = is_command;
            }
            accumulated_args.push(arg.clone());
        }

        // !! the following is duplicated from the above to handle the case of only one command
        // !! TODO: maybe better to refactor as "do while" loop
        let creator = command_creator
            .take()
            .ok_or("Should have at least one command")?;

        let (executor, progress) = executor_builder.create(accumulated_args, creator)?;
        executors.push(executor);
        progresses.push(progress);
        Ok((executors, progresses))
    }

    fn if_at_least_one_command(first_arg: &str) -> bool {
        subcommand(first_arg).is_some()
    }
}

//TODO: update this soon
#[derive(Parser)]
enum VVSubCommand {
    #[clap(name = "convert")]
    Convert(convert::Args),
    #[clap(name = "write")]
    Write(write::Args),
    #[clap(name = "read")]
    Read(read::Args),
    #[clap(name = "render")]
    Render(render::Args),
    #[clap(name = "metrics")]
    Metrics(metrics::Args),
    #[clap(name = "downsample")]
    Downsample(downsample::Args),
    #[clap(name = "upsample")]
    Upsample(upsample::Args),
    #[clap(name = "normal")]
    NormalEstimation(normal_estimation::Args),
    #[clap(name = "info")]
    Info(info::Args),
    #[clap(name = "dash")]
    Dash(dash::Args),
    #[clap(name = "extend")]
    Extend(extension::Args),
}

fn display_main_help_msg() {
    let _subcommand = VVSubCommand::parse_from(&["vv", "--help"]);
}

#[cfg(test)]
mod pipeline_mod_test {
    use super::*;

    #[test]
    fn if_at_least_one_command_test() {
        assert!(Pipeline::if_at_least_one_command("read"));
        assert!(Pipeline::if_at_least_one_command("write"));
        assert!(Pipeline::if_at_least_one_command("render"));
        assert!(Pipeline::if_at_least_one_command("metrics"));
        assert!(Pipeline::if_at_least_one_command("downsample"));
        assert!(Pipeline::if_at_least_one_command("upsample"));
        assert!(Pipeline::if_at_least_one_command("convert"));
        assert!(!Pipeline::if_at_least_one_command("not_a_command"));
    }
}
