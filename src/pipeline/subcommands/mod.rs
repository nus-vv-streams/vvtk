mod play;
mod read;
mod to_png;
mod write;

use std::sync::mpsc::Sender;

pub use play::Play;
pub use read::Read;
pub use to_png::ToPng;
pub use write::Write;

use super::PipelineMessage;

pub trait Subcommand {
    fn handle(&mut self, message: PipelineMessage, out: &Sender<PipelineMessage>);
}
