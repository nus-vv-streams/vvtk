use crate::render::wgpu::camera::CameraPosition;
use crate::render::wgpu::reader::FrameRequest;
use crate::PCMetadata;

/** 
 * This file contains all FetchRequest struct and related implementation
 */


#[derive(Debug, Clone, Copy)]
pub struct FetchRequest {
    pub object_id: u8,
    // pub quality: u8,
    /// Frame offset from the start of the video.
    ///
    /// To get the frame number, add the offset to the frame number of the first frame in the video.
    pub frame_offset: u64,
    /// The camera position when the frame was requested.
    pub camera_pos: Option<CameraPosition>,
    pub buffer_occupancy: usize,
}

impl FetchRequest {
    pub fn new(req: FrameRequest, buffer_occupancy: usize) -> Self {
        FetchRequest {
            object_id: req.object_id,
            frame_offset: req.frame_offset,
            camera_pos: req.camera_pos,
            buffer_occupancy,
        }
    }
}

impl From<FetchRequest> for PCMetadata {
    fn from(val: FetchRequest) -> Self {
        PCMetadata {
            object_id: val.object_id,
            frame_offset: val.frame_offset,
        }
    }
}

impl From<FetchRequest> for FrameRequest {
    fn from(val: FetchRequest) -> Self {
        FrameRequest {
            object_id: val.object_id,
            frame_offset: val.frame_offset,
            camera_pos: val.camera_pos,
        }
    }
}
