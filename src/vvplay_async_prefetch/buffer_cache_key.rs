use crate::render::wgpu::reader::FrameRequest;
use crate::PCMetadata;
/**
 * This file contains the BufferCacheKey for vvplay_async_prefetch.
 */

//t: what is this for?
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct BufferCacheKey {
    pub object_id: u8,
    pub frame_offset: u64,
}

impl From<FrameRequest> for BufferCacheKey {
    fn from(req: FrameRequest) -> Self {
        Self {
            object_id: req.object_id,
            frame_offset: req.frame_offset,
        }
    }
}

impl From<PCMetadata> for BufferCacheKey {
    fn from(metadata: PCMetadata) -> Self {
        Self {
            object_id: metadata.object_id,
            frame_offset: metadata.frame_offset,
        }
    }
}

