mod downsample;
mod metrics;
mod read;
mod to_png;
mod upsample;
mod write;

pub use downsample::Downsampler;
pub use metrics::MetricsCalculator;
pub use read::Read;
pub use to_png::ToPng;
pub use upsample::Upsampler;
pub use write::Write;

use super::{channel::Channel, PipelineMessage};

pub trait Subcommand {
    fn handle(&mut self, messages: Vec<PipelineMessage>, out: &Channel);
}
