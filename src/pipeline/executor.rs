use super::{
    channel::Channel, subcommands::Subcommand, PipelineMessage, Progress, SubcommandCreator,
};
use crossbeam_channel::{unbounded, Receiver};
use std::collections::HashSet;

pub struct Executor {
    //subcommand name
    name: String,
    input_stream_names: Vec<String>,
    output_name: String,
    external_args_option: Option<Vec<String>>,
    //t: what is this input here for?
    inputs: Vec<Receiver<PipelineMessage>>,
    channel: Channel,
    handler: Box<dyn Subcommand>,
}

pub struct ExecutorBuilder {
    output_stream_names: HashSet<String>,
}

impl ExecutorBuilder {
    pub fn new() -> Self {
        ExecutorBuilder {
            output_stream_names: HashSet::new(),
        }
    }

    //t: this function will identify what to do with the args passed in, and create a Executor that has a channel inside it for the progress
    //t: change this part to pass in the argument for external subcommand
    pub fn create(
        &mut self,
        args: Vec<String>,
        creator: SubcommandCreator,
    ) -> Result<(Executor, Receiver<Progress>), String> {
        //t: make sure that the command name exists
        let name = match args.first() {
            Some(command_name) => command_name.clone(),
            None => return Err("Should have command name".to_string()),
        };

        let mut inner_args = Vec::new();
        let mut external_args = Vec::new();
        let mut input_stream_names = Vec::new();
        let mut output_name = "".to_string();

        let cmd = args[0].clone();

        let mut has_input = false;
        let mut has_help = false;
        let mut has_external_arg = false;
        // println!("args: {:?}", args);
        for arg in args {
            //t: help part
            if arg.eq("--help") || arg.eq("-h") {
                has_help = true;
            }

            //t: this is for the input
            if arg.starts_with("+input") || arg.starts_with("+in") {
                //t: take the element after =
                let input_streams = match arg.split("=").nth(1) {
                    Some(input_streams) => input_streams,
                    None => return Err("Expected name of input stream".to_string()),
                };

                //t: handle the input stream names that contains ,
                for input_name in input_streams.split(',') {
                    // check if input stream name is in the set, panic if not
                    if !self.output_stream_names.contains(input_name) {
                        // get the existing output stream names, concat them with ", "
                        let existing_output_stream_names = self
                            .output_stream_names
                            .iter()
                            .map(|s| format!("`{}`", s))
                            .collect::<Vec<String>>()
                            .join(", ");

                        return Err(format!(
                            "No output stream with name `{}` found, existing outputs are {}",
                            input_streams, existing_output_stream_names
                        ));
                    } else {
                        input_stream_names.push(input_name.to_string());
                    }
                }
                has_input = true;
            } else if arg.starts_with("+xargs") {
                //TODO: check if it is external subcommand
                let external_args_str: &str = match arg.split("=").nth(1) {
                    Some(external_args_str) => external_args_str,
                    None => return Err("Expected arguments for external subcommand".to_string()),
                };
                has_external_arg = true;
                for arg in external_args_str.split(',') {
                    println!("the xargs are {:?}", &arg);
                    external_args.push(arg.to_string());
                }   
                
            } else if arg.starts_with("+output") || arg.starts_with("+out") {
                //t: handle the output stream here
                output_name = match arg.split('=').nth(1) {
                    Some(output_name) => output_name.to_string(),
                    None => return Err("Expected name of output stream".to_string()),
                };

                self.output_stream_names.insert(output_name.clone());
            } else {
                //t: if it is not input or output, then it will be classified as inner_args
                // TODO: command line argument of external subcommand will fall under here
                inner_args.push(arg);
            }
        }

        //t: certain command will not need to have an input, if not, throw an error 
        if has_input
            || cmd.as_str() == "read"
            || cmd.as_str() == "convert"
            || cmd.as_str() == "info"
            || cmd.as_str() == "dash"
            || cmd.as_str() == "extend"
            || has_help
        {
        } else {
            return Err(format!(
                "`{}` needs to consume an input, but no named input is found, specify it using `+input=input_name`",
                cmd.as_str()
            ));
        }
         // Check if there is external args
         let external_args_option: Option<Vec<String>>;
         if has_external_arg {
            external_args_option = Some(external_args);
         } else {
            external_args_option = None;
         }

        //t: pass in inner arg to subcommand here?
        let handler = creator(inner_args);

        //t: what is the progress here for
        //t: create a channel here, and passed the channel receiver as a result, Progress can either be Incr or Completed
        let (progress_tx, progress_rx) = unbounded();
        //t: create a pipeline spefic channel with the sender here to do ???
        let channel = Channel::new(progress_tx);
        //t: pass the channel to the executor
        let executor = Executor {
            name,
            input_stream_names,
            output_name,
            external_args_option,
            inputs: vec![],
            channel,
            handler,
        };
        Ok((executor, progress_rx))
    }
}

unsafe impl Send for Executor {}

impl Executor {
    //t: what's the point of having two create
    #[allow(dead_code)]
    pub fn create(args: Vec<String>, creator: SubcommandCreator) -> (Self, Receiver<Progress>) {
        let name = args.first().expect("Should have command name").clone();
        let mut inner_args = Vec::new();
        let mut input_stream_names = Vec::new();
        let mut output_name = "".to_string();
        for arg in args {
            if arg.starts_with("+input") || arg.starts_with("+in") {
                let input_streams = arg
                    .split('=')
                    .nth(1)
                    .expect("Expected name of input stream");
                for input_name in input_streams.split(',') {
                    input_stream_names.push(input_name.to_string());
                }
            } else if arg.starts_with("+output") || arg.starts_with("+out") {
                output_name = arg
                    .split('=')
                    .nth(1)
                    .expect("Expected name of output stream")
                    .to_string();
            } else {
                inner_args.push(arg);
            }
        }
        let handler = creator(inner_args);

        let (progress_tx, progress_rx) = unbounded();
        let channel = Channel::new(progress_tx);
        //TODO: not sure why this part exists, but just use this first, if it is actually used need to move the code here also
        let executor = Self {
            name,
            input_stream_names,
            output_name,
            external_args_option: None,
            inputs: vec![],
            channel,
            handler,
        };
        (executor, progress_rx)
    }

    pub fn input_names(&self) -> Vec<String> {
        self.input_stream_names.clone()
    }

    pub fn output_name(&self) -> &str {
        &self.output_name
    }

    pub fn output(&mut self) -> Receiver<PipelineMessage> {
        self.channel.subscribe()
    }

    pub fn set_inputs(&mut self, inputs: Vec<Receiver<PipelineMessage>>) {
        self.inputs = inputs;
    }

    pub fn run(self) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || self.start())
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    fn start(mut self) {
        //t: why this part use the handler when the input is empty, for some command, the input can be empty tho?
        if self.inputs.is_empty() {
            self.handler.handle(vec![], &self.channel, &self.external_args_option);
            return;
        }
        //t: this part receives all the message from the input channel, and do something to each message
        while let Ok(messages) = self
            .inputs
            .iter()
            .map(|recv| recv.recv())
            .collect::<Result<Vec<PipelineMessage>, _>>()
        {
            //t: so if one of the provider sent the end message, this process will end
            let should_break = messages.iter().any(|message| {
                if let PipelineMessage::End = message {
                    true
                } else {
                    false
                }
            });

            // the handle is called here
            self.handler.handle(messages, &self.channel, &self.external_args_option);

            if should_break {
                break;
            }
        }
    }
}
