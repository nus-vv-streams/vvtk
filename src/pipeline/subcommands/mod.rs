pub mod convert;
pub mod downsample;
pub mod info;
pub mod metrics;
pub mod play;
pub mod read;
pub mod reconstruct;
pub mod render;
pub mod upsample;
pub mod write;

pub use convert::Convert;
pub use downsample::Downsampler;
pub use info::Info;
pub use metrics::MetricsCalculator;
pub use play::Play;
pub use read::Read;
pub use reconstruct::Reconstructer;
pub use render::Render;
pub use upsample::Upsampler;
pub use write::Write;

use super::{channel::Channel, PipelineMessage};

pub trait Subcommand {
    fn handle(&mut self, messages: Vec<PipelineMessage>, out: &Channel);
}
