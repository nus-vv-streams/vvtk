mod metrics;
mod read;
mod to_png;
mod write;

use std::sync::mpsc::Sender;

pub use metrics::Metrics;
pub use read::Read;
pub use to_png::ToPng;
pub use write::Write;

use super::{channel::Channel, PipelineMessage, Progress};

pub trait Subcommand {
    fn handle(
        &mut self,
        messages: Vec<PipelineMessage>,
        out: &Channel,
        progress: &Sender<Progress>,
    );
}
