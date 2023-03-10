#[cfg(feature = "render")]
pub mod wgpu;

#[cfg(not(feature = "render"))]
pub mod wgpu {
    pub mod png {
        use std::{ffi::OsString, marker::PhantomData};

        use crate::formats::{pointxyzrgba::PointXyzRgba, PointCloud};

        pub struct PngWriter<'a> {
            data: PhantomData<&'a u32>,
        }

        impl<'a> PngWriter<'a> {
            pub fn new(
                output_dir: OsString,
                camera_x: f32,
                camera_y: f32,
                camera_z: f32,
                camera_yaw: f32,
                camera_pitch: f32,
                width: u32,
                height: u32,
            ) -> Self {
                Self {
                    data: PhantomData::default(),
                }
            }

            pub fn write_to_png(&mut self, pc: &PointCloud<PointXyzRgba>) {}
        }
    }

    pub mod builder {
        use super::renderer::Renderer;

        pub struct RenderBuilder;

        pub trait Windowable {}

        impl RenderBuilder {
            pub fn default() -> Self {
                Self
            }

            pub fn add_window<T: Windowable>(&self, renderer: T) {}

            pub fn get_windowed_mut(&self, _: ()) -> Result<Dummy, ()> {
                Ok(Dummy)
            }

            pub fn run(&self) {}
        }

        pub struct Dummy;
        impl Dummy {
            pub fn add_output(&self, _: ()) {}
        }
    }

    pub mod camera {
        use cgmath::{Point3, Rad};

        pub struct Camera;

        impl Camera {
            pub fn new<V: Into<Point3<f32>>, Y: Into<Rad<f32>>, P: Into<Rad<f32>>>(
                position: V,
                yaw: Y,
                pitch: P,
            ) -> Self {
                Camera
            }
        }
    }

    pub mod metrics_reader;

    pub mod controls {
        use super::builder::Windowable;

        pub struct Controller {
            pub slider_end: usize,
        }

        impl Windowable for Controller {}
    }

    pub mod reader {
        use crate::pcd::PointCloudData;

        pub trait Reader {}

        pub struct BufRenderReader;

        impl BufRenderReader {
            pub fn new<T: Reader>(size: usize, reader: T) -> Self {
                Self
            }
        }

        pub struct RenderReader;

        impl Reader for BufRenderReader {}
        impl Reader for RenderReader {}
    }

    pub mod renderer {
        use super::{
            builder::Windowable, camera::Camera, metrics_reader::MetricsReader, reader::Reader,
        };

        pub struct Renderer;

        impl Renderer {
            pub fn new<T: Reader>(
                reader: T,
                fps: f32,
                camera: Camera,
                (width, height): (u32, u32),
                metrics_reader: Option<MetricsReader>,
            ) -> Self {
                Self
            }
        }

        impl Windowable for Renderer {}
    }
}
