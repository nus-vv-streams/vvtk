//! # Vivo Toolkit
//#[warn(missing_docs)]

pub mod abr;
pub mod codec;
pub mod dash;
pub mod downsample;
pub mod estimatethroughput;
pub mod formats;
pub mod lodify;
pub mod metrics;
pub mod pcd;
pub mod pipeline;
pub mod player;
pub mod ply;
pub mod reconstruct;
pub mod render;
pub mod simulation;
pub mod upsample;
pub mod utils;
pub mod velodyne;
pub mod vvplay_async_prefetch;

use dash::fetcher::FetchResult;
use formats::{pointxyzrgba::PointXyzRgba, PointCloud};

use render::wgpu::reader::FrameRequest;

#[derive(Debug)]
pub enum BufMsg {
    PointCloud(
        (
            PCMetadata,
            tokio::sync::mpsc::UnboundedReceiver<PointCloud<PointXyzRgba>>,
        ),
    ),
    /// Fetch result from the fetcher
    FetchDone((FrameRequest, FetchResult)),
    // FetchDone(FrameRequest),
    FrameRequest(FrameRequest),
}

/// Metadata for point cloud. Used in BufMsg.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct PCMetadata {
    pub object_id: u8,
    pub frame_offset: u64,
}

impl From<PCMetadata> for FrameRequest {
    fn from(val: PCMetadata) -> Self {
        FrameRequest {
            object_id: val.object_id,
            frame_offset: val.frame_offset,
            // TODO: fix this once PCMetadata is updated
            camera_pos: None,
        }
    }
}
