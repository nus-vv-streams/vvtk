mod metrics;
mod read;
pub mod to_png;
pub mod write;
mod convert;
mod play;

use std::sync::mpsc::Sender;

pub use metrics::Metrics;
pub use read::Read;
pub use to_png::ToPng;
pub use write::Write;
pub use convert::Convert;

use super::{PipelineMessage, Progress};

pub trait Subcommand {
    fn handle(
        &mut self,
        message: PipelineMessage,
        out: &Sender<PipelineMessage>,
        progress: &Sender<Progress>,
    );
}