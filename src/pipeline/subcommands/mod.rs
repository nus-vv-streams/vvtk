mod downsample;
mod metrics;
mod read;
pub mod to_png;
pub mod write;
pub mod upsample;

pub use downsample::Downsampler;
pub use metrics::MetricsCalculator;
pub use read::Read;
pub use to_png::ToPng;
pub use upsample::Upsampler;
pub use write::Write;
// pub use convert::Convert;

use super::{channel::Channel, PipelineMessage};

pub trait Subcommand {
    fn handle(&mut self, messages: Vec<PipelineMessage>, out: &Channel);
}