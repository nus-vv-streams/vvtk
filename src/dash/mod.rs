use crate::render::wgpu::camera::CameraPosition;

pub mod buffer;
pub mod fetcher;
pub mod parser;

pub trait ViewportPrediction: Send {
    fn add(&mut self, pos: Option<CameraPosition>);
    fn predict(&self) -> Option<CameraPosition>;
}

pub trait ThroughputPrediction: Send {
    fn add(&mut self, throughput: f64);
    fn predict(&self) -> Option<f64>;
}
