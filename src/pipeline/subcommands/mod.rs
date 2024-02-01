pub mod convert;
pub mod dash;
pub mod downsample;
pub mod info;
pub mod metrics;
pub mod normal_estimation;
pub mod read;
pub mod render;
pub mod upsample;
pub mod write;
pub mod extension;

pub use convert::Convert;
pub use dash::Dash;
pub use downsample::Downsampler;
pub use info::Info;
pub use metrics::MetricsCalculator;
pub use normal_estimation::NormalEstimation;
pub use read::Read;
pub use render::Render;
pub use upsample::Upsampler;
pub use write::Write;
pub use extension::Extension;

use super::{channel::Channel, PipelineMessage};

pub trait Subcommand {
    //t: messages is input, out is output?
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel, external_args: &Option<Vec<String>>);
}
