pub mod convert;
pub mod dash;
pub mod downsample;
pub mod info;
pub mod lodify;
pub mod metrics;
pub mod normal_estimation;
pub mod read;
pub mod render;
pub mod subsample;
pub mod upsample;
pub mod write;

pub use convert::Convert;
pub use dash::Dash;
pub use downsample::Downsampler;
pub use info::Info;
pub use lodify::Lodifier;
pub use metrics::MetricsCalculator;
pub use normal_estimation::NormalEstimation;
pub use read::Read;
pub use render::Render;
pub use subsample::Subsampler;
pub use upsample::Upsampler;
pub use write::Write;

use super::{channel::Channel, PipelineMessage};

pub trait Subcommand {
    fn handle(&mut self, messages: Vec<PipelineMessage>, out: &Channel);
}
