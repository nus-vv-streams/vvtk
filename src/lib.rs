//! # Vivo Toolkit
//#[warn(missing_docs)]

pub mod codec;
#[cfg(feature = "dash")]
pub mod dash;
pub mod downsample;
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
#[derive(Debug, Clone)]
pub enum BufMsg {
    PointCloud((PCMetadata, PointCloud<PointXyzRgba>)),
    FrameRequest(FrameRequest),
}

/// Metadata for point cloud. Used in BufMsg.
///
/// Includes statistics for the point cloud
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct PCMetadata {
    pub object_id: u8,
    pub frame_offset: u64,
    pub last5_avg_bitrate: usize,
}

impl From<PCMetadata> for FrameRequest {
    fn from(val: PCMetadata) -> Self {
        FrameRequest {
            object_id: val.object_id,
            frame_offset: val.frame_offset,
        }
    }
}

impl From<FrameRequest> for PCMetadata {
    fn from(val: FrameRequest) -> Self {
        PCMetadata {
            object_id: val.object_id,
            frame_offset: val.frame_offset,
            last5_avg_bitrate: 0,
        }
    }
}
