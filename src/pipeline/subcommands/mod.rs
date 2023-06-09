pub mod convert;
pub mod downsample;
pub mod metrics;
pub mod play;
pub mod read;
pub mod to_png;
pub mod upsample;
pub mod write;

pub use convert::Convert;
pub use downsample::Downsampler;
pub use metrics::MetricsCalculator;
pub use play::Play;
pub use read::Read;
pub use to_png::ToPng;
pub use upsample::Upsampler;
pub use write::Write;

use super::{channel::Channel, PipelineMessage};

pub trait Subcommand {
    fn handle(&mut self, messages: Vec<PipelineMessage>, out: &Channel);
}
