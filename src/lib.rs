//! # Vivo Toolkit
//#[warn(missing_docs)]

pub mod abr;
pub mod codec;
#[cfg(feature = "dash")]
pub mod dash;
pub mod downsample;
pub mod estimatethroughput;
pub mod formats;
pub mod metrics;
pub mod pcd;
pub mod pipeline;
pub mod ply;
pub mod render;
pub mod upsample;
pub mod utils;

use formats::{pointxyzrgba::PointXyzRgba, PointCloud};
use render::wgpu::reader::FrameRequest;

/// Message types sent to the Buffer Manager of ply_play
#[derive(Debug)]
pub enum BufMsg {
    /// Point cloud message.
    ///
    /// Contains the point cloud and the metadata info for the point cloud.
    PointCloud(
        (
            PCMetadata,
            tokio::sync::mpsc::UnboundedReceiver<PointCloud<PointXyzRgba>>,
        ),
    ),
    /// Frame request message.
    FrameRequest(FrameRequest),
}

/// Metadata for point cloud. Used in BufMsg.
///
/// Includes statistics for the point cloud
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

impl From<FrameRequest> for PCMetadata {
    fn from(val: FrameRequest) -> Self {
        PCMetadata {
            object_id: val.object_id,
            frame_offset: val.frame_offset,
        }
    }
}
