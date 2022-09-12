use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::formats::PointCloud;
use crate::pipeline::{PipelineMessage, Progress};
use crate::render::wgpu::camera::{Camera, CameraState};
use crate::render::wgpu::renderer::PointCloudRenderer;
use clap::Parser;
use std::ffi::OsString;
use std::num::NonZeroU32;
use std::path::Path;
use std::sync::mpsc::Sender;
use wgpu::{Buffer, Device, Queue, Texture, TextureDescriptor, TextureView};
use winit::dpi::PhysicalSize;

use super::Subcommand;

/// Converts a folder of .pcd files to a folder of .png images
#[derive(Parser)]
struct Args {
    /// Directory to store output png images
    #[clap(short, long)]
    output_dir: OsString,
    /// Number of pcd files to convert
    #[clap(short = 'n', long)]
    frames: Option<usize>,
    #[clap(short = 'x', long, default_value_t = 0.0)]
    camera_x: f32,
    #[clap(short = 'y', long, default_value_t = 0.0)]
    camera_y: f32,
    #[clap(short = 'z', long, default_value_t = 1.3)]
    camera_z: f32,
    #[clap(long = "yaw", default_value_t = -90.0)]
    camera_yaw: f32,
    #[clap(long = "pitch", default_value_t = 0.0)]
    camera_pitch: f32,
    #[clap(short, long, default_value_t = 1600)]
    width: u32,
    #[clap(short, long, default_value_t = 900)]
    height: u32,
}

pub struct ToPng<'a> {
    output_dir: OsString,
    size: PhysicalSize<u32>,
    device: Device,
    queue: Queue,
    texture_desc: TextureDescriptor<'a>,
    texture: Texture,
    texture_view: TextureView,
    u32_size: u32,
    output_buffer: Buffer,
    camera_state: CameraState,
    point_renderer: Option<PointCloudRenderer<PointCloud<PointXyzRgba>>>,
    count: usize,
}

impl<'a> ToPng<'a> {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        let args: Args = Args::parse_from(args);
        let output_path = Path::new(&args.output_dir);

        std::fs::create_dir_all(output_path).expect("Failed to create output directory");

        let size = PhysicalSize::new(args.width, args.height);
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default(), None)).unwrap();

        let texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
        };
        let texture = device.create_texture(&texture_desc);
        let texture_view = texture.create_view(&Default::default());

        let u32_size = std::mem::size_of::<u32>() as u32;

        let output_buffer_size = (u32_size * size.width * size.height) as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: None,
            mapped_at_creation: false,
        };
        let output_buffer = device.create_buffer(&output_buffer_desc);

        let camera = Camera::new(
            (args.camera_x, args.camera_y, args.camera_z),
            cgmath::Deg(args.camera_yaw),
            cgmath::Deg(args.camera_pitch),
        );
        let camera_state = CameraState::new(camera, size.width, size.height);

        Box::from(ToPng {
            output_dir: args.output_dir,
            size,
            device,
            queue,
            texture_desc,
            texture,
            texture_view,
            u32_size,
            output_buffer,
            camera_state,
            point_renderer: None,
            count: 0,
        })
    }
}

impl Subcommand for ToPng<'_> {
    fn handle(
        &mut self,
        message: PipelineMessage,
        out: &Sender<PipelineMessage>,
        progress: &Sender<Progress>,
    ) {
        match &message {
            PipelineMessage::PointCloud(pc) => {
                if self.point_renderer.is_none() {
                    self.point_renderer = Some(PointCloudRenderer::new(
                        &self.device,
                        self.texture_desc.format,
                        pc,
                        self.size,
                        &self.camera_state,
                    ));
                }

                let point_renderer = self.point_renderer.as_mut().unwrap();
                point_renderer.update_vertices(&self.device, &self.queue, pc);
                let mut encoder = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                point_renderer.render(&mut encoder, &self.texture_view);
                encoder.copy_texture_to_buffer(
                    wgpu::ImageCopyTexture {
                        aspect: wgpu::TextureAspect::All,
                        texture: &self.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                    },
                    wgpu::ImageCopyBuffer {
                        buffer: &self.output_buffer,
                        layout: wgpu::ImageDataLayout {
                            offset: 0,
                            bytes_per_row: NonZeroU32::new(self.u32_size * self.size.width),
                            rows_per_image: NonZeroU32::new(self.size.height),
                        },
                    },
                    self.texture_desc.size,
                );

                self.queue.submit(Some(encoder.finish()));
                {
                    let buffer_slice = self.output_buffer.slice(..);
                    let mapping = buffer_slice.map_async(wgpu::MapMode::Read);
                    self.device.poll(wgpu::Maintain::Wait);
                    pollster::block_on(mapping).unwrap();

                    let data = buffer_slice.get_mapped_range();

                    use image::{ImageBuffer, Rgba};
                    let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(
                        self.size.width,
                        self.size.height,
                        data,
                    )
                    .unwrap();

                    let filename = format!("{}.png", self.count);
                    self.count += 1;
                    let output_path = Path::new(&self.output_dir);
                    buffer.save(output_path.join(Path::new(&filename))).unwrap();
                }
                self.output_buffer.unmap();
                progress.send(Progress::Incr);
            }
            PipelineMessage::End => {
                progress.send(Progress::Completed);
            }
        }
        out.send(message);
    }
}
