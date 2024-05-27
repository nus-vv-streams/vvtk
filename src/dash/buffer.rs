use crate::{
    formats::{pointxyzrgba::PointXyzRgba, PointCloud},
    render::wgpu::reader::FrameRequest,
};
use std::{
    collections::VecDeque,
    fmt::{Debug, Formatter},
};

/// A frame request with its status
pub struct RequestStatus {
    pub req: FrameRequest,
    pub state: FrameStatus,
}

impl Debug for RequestStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{{ object_id: {}, frame_offset: {}, state: {:?} }}",
            self.req.object_id, self.req.frame_offset, self.state
        )
    }
}

/// At what state is the frame in?
pub enum FrameStatus {
    /// Frame is being fetched by the fetcher
    Fetching,
    /// Frame is being decoded by the decoder
    Decoding,
    /// Frame is being ready to be rendered
    Ready(
        usize, // remaining frames in the channel
        tokio::sync::mpsc::UnboundedReceiver<PointCloud<PointXyzRgba>>,
    ),
}

impl Debug for FrameStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameStatus::Fetching => write!(f, "Fetching"),
            FrameStatus::Decoding => write!(f, "Decoding"),
            FrameStatus::Ready(remaining, _) => {
                write!(f, "Ready ({remaining} remaining)")
            }
        }
    }
}

#[derive(Debug)]
pub struct Buffer {
    frames: VecDeque<RequestStatus>,
    capacity: usize,
}

impl Buffer {
    pub fn new(capacity: usize) -> Self {
        Buffer {
            frames: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    #[inline]
    /// Pushes back a new frame request with fetching status
    pub fn add(&mut self, req: FrameRequest) {
        assert!(self.frames.len() <= self.capacity);
        self.frames.push_back(RequestStatus {
            req,
            state: FrameStatus::Fetching,
        });
    }

    #[inline]
    pub fn push_back(&mut self, state: RequestStatus) {
        assert!(self.frames.len() <= self.capacity);
        self.frames.push_back(state);
    }

    #[inline]
    pub fn push_front(&mut self, state: RequestStatus) {
        assert!(self.frames.len() <= self.capacity);
        self.frames.push_front(state);
    }

    #[inline]
    pub fn pop_front(&mut self) -> Option<RequestStatus> {
        self.frames.pop_front()
    }

    #[inline]
    pub fn front(&self) -> Option<&RequestStatus> {
        self.frames.front()
    }

    #[inline]
    pub fn back(&self) -> Option<&RequestStatus> {
        self.frames.back()
    }

    #[inline]
    /// Update the request and state of a frame. Panics if key is not found. If the new_state is Ready(0, _), the frame is removed.
    pub fn update(&mut self, key: FrameRequest, new_key: FrameRequest, new_state: FrameStatus) {
        let idx = self
            .frames
            .iter()
            .position(|f| f.req == key)
            .expect("Frame not found");
        if let FrameStatus::Ready(0, _) = new_state {
            self.frames.remove(idx);
            return;
        }
        self.frames[idx].req = new_key;
        self.frames[idx].state = new_state;
    }

    #[inline]
    /// Update only the state of a frame. Panics if key is not found. If the state is Ready(0, _), the frame is removed.
    pub fn update_state(&mut self, req: FrameRequest, state: FrameStatus) {
        let idx = self
            .frames
            .iter()
            .position(|f| f.req == req)
            .expect("Frame not found");
        if let FrameStatus::Ready(0, _) = state {
            self.frames.remove(idx);
            return;
        }
        self.frames[idx].state = state;
    }

    pub fn get(&self, req: FrameRequest) -> Option<&RequestStatus> {
        self.frames.iter().find(|f| f.req == req)
    }

    pub fn get_mut(&mut self, req: FrameRequest) -> Option<&mut RequestStatus> {
        self.frames.iter_mut().find(|f| f.req == req)
    }

    pub fn remove(&mut self, req: FrameRequest) {
        let idx = self
            .frames
            .iter()
            .position(|f| f.req == req)
            .expect("Frame not found");
        self.frames.remove(idx);
    }

    pub fn is_full(&self) -> bool {
        self.frames.len() >= self.capacity
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Returns the number of requests that are ready to be rendered
    pub fn len(&self) -> usize {
        self.frames
            .iter()
            .map(|f| match f.state {
                FrameStatus::Ready(_, _) => 1,
                _ => 0,
            })
            .sum()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn iter(&self) -> impl Iterator<Item = &RequestStatus> {
        self.frames.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut RequestStatus> {
        self.frames.iter_mut()
    }

    pub fn clear(&mut self) {
        self.frames.clear();
    }

    pub fn is_frame_in_buffer(&self, req: FrameRequest) -> bool {
        // This implementation assumes that the frame index stored in the buffer form contiguous sequence.
        // If the first frame offset is 2, last frame offset is 5, then frame 3, 4 will also exist in current buffer.
        if req.frame_offset >= self.front().unwrap().req.frame_offset
            && req.frame_offset <= self.back().unwrap().req.frame_offset
        {
            return true;
        }
        false
    }
}
