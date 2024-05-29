use crate::formats::pointxyzrgba::PointXyzRgba;
use crate::formats::PointCloud;
use crate::render::wgpu::camera::{Camera, CameraState};
use crate::render::wgpu::renderer::{parse_wgpu_color, PointCloudRenderer};
// use color_space::Rgb;
use std::ffi::OsString;
use std::num::NonZeroU32;
use std::path::Path;
use std::str::FromStr;
use wgpu::{Buffer, Device, InstanceDescriptor, Queue, Texture, TextureDescriptor, TextureView};
use winit::dpi::PhysicalSize;

use super::camera::CameraPosition;
use std::process::{Command, Stdio};

#[derive(clap::ValueEnum, Debug, Copy, Clone, Eq, PartialEq)]
pub enum RenderFormat {
    Png,
    Mp4,
}

impl ToString for RenderFormat {
    fn to_string(&self) -> String {
        match self {
            RenderFormat::Png => "png".to_string(),
            RenderFormat::Mp4 => "mp4".to_string(),
        }
    }
}

impl FromStr for RenderFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "png" => Ok(RenderFormat::Png),
            "mp4" => Ok(RenderFormat::Mp4),
            _ => Err("Invalid render format".to_string()),
        }
    }
}

pub struct PngWriter<'a> {
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
    background_color: Option<wgpu::Color>,
    // count: usize,
    // bg_color: Rgb,
    render_format: RenderFormat,
}

impl<'a> PngWriter<'a> {
    pub fn new(
        output_dir: OsString,
        camera_x: f32,
        camera_y: f32,
        camera_z: f32,
        camera_yaw: cgmath::Rad<f32>,
        camera_pitch: cgmath::Rad<f32>,
        width: u32,
        height: u32,
        bg_color: &str,
        render_format: RenderFormat,
    ) -> Self {
        let output_path = Path::new(&output_dir);

        std::fs::create_dir_all(output_path).expect("Failed to create output directory");

        let size = PhysicalSize::new(width, height);
        let instance = wgpu::Instance::new(InstanceDescriptor::default());
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
            view_formats: &[],
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

        let camera = Camera::new((camera_x, camera_y, camera_z), camera_yaw, camera_pitch);
        let camera_state = CameraState::new(camera, size.width, size.height);
        Self {
            output_dir,
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
            // background_color: None,
            background_color: parse_wgpu_color(bg_color).ok(),
            // bg_color: parse_bg_color(bg_color).unwrap(),
            // count: 0,
            render_format,
        }
    }

    /// Set the background color. Call this function before the first [`write_to_png`] call
    ///
    /// [`write_to_png`]: #method.write_to_png
    pub fn set_background_color(&mut self, color: wgpu::Color) {
        self.background_color = Some(color);
    }

    pub fn render_format(&self) -> RenderFormat {
        self.render_format
    }

    /// Update the camera position
    pub fn update_camera_pos(&mut self, pos: CameraPosition) {
        self.camera_state.update_camera_pos(pos);
        if let Some(ref mut renderer) = self.point_renderer {
            renderer.update_camera(&self.queue, self.camera_state.camera_uniform);
        }
    }

    pub fn write_to_png(&mut self, pc: &PointCloud<PointXyzRgba>, filename: &str) {
        if self.point_renderer.is_none() {
            let renderer = PointCloudRenderer::new(
                &self.device,
                self.texture_desc.format,
                pc,
                self.size,
                &self.camera_state,
                self.background_color.unwrap_or(wgpu::Color::BLACK),
            );
            self.point_renderer = Some(if let Some(color) = self.background_color {
                renderer.with_background_color(color)
            } else {
                renderer
            })
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
            buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
            self.device.poll(wgpu::Maintain::Wait);

            let data = buffer_slice.get_mapped_range();

            use image::{ImageBuffer, Rgba};
            let buffer =
                ImageBuffer::<Rgba<u8>, _>::from_raw(self.size.width, self.size.height, data)
                    .unwrap();

            let output_path = Path::new(&self.output_dir);
            buffer.save(output_path.join(Path::new(&filename))).unwrap();
        }
        self.output_buffer.unmap();
    }

    pub fn write_to_mp4(&self, name_length: u32, fps: f32, verbose: bool) {
        let img_dir_path = Path::new(&self.output_dir);
        let mp4_save_path = img_dir_path.parent().unwrap();
        let mut mp4_path = mp4_save_path.to_path_buf();
        mp4_path.push("output.mp4");

        PngWriter::png_to_mp4(img_dir_path, &mp4_path, name_length, fps, verbose);

        // delete tmp png dir
        std::fs::remove_dir_all(img_dir_path).unwrap();
    }

    pub fn png_to_mp4(img_dir: &Path, mp4_path: &Path, name_length: u32, fps: f32, verbose: bool) {
        let tmp_png_dir = Path::new(img_dir);
        // mp4 dir is parent of tmp_png_dir
        // read all png file in tmp_png_dir, then sort them lexicographically, then write to mp4
        let mut png_files: Vec<_> = std::fs::read_dir(tmp_png_dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        png_files.sort();

        // use ffmpeg to convert png to mp4
        let cmd = Command::new("ffmpeg")
            .arg("-y")
            .arg("-framerate")
            .arg(fps.to_string())
            .arg("-i")
            .arg(format!("{}/%0{}d.png", tmp_png_dir.display(), name_length))
            .arg("-c:v")
            .arg("libx264")
            .arg("-r")
            .arg(fps.to_string())
            // .arg("-pix_fmt")
            // .arg("yuv420p")
            .arg(mp4_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start ffmpeg process");

        if verbose {
            println!("cmd is {:?}", cmd);
        }

        let output = cmd
            .wait_with_output()
            .expect("Failed to run/wait ffmpeg process");

        if output.status.success() {
            if verbose {
                let mut output_string = String::new();
                if !output.stdout.is_empty() {
                    output_string
                        .push_str(String::from_utf8_lossy(&output.stdout).to_string().as_str());
                }
                // ffmpeg output all of its logging data to stderr
                if !output.stderr.is_empty() {
                    output_string
                        .push_str(String::from_utf8_lossy(&output.stderr).to_string().as_str());
                }
                println!("ffmpeg:\n{}", output_string);
                println!("mp4 file is saved to {}", mp4_path.display());
            }
        } else {
            eprintln!("ffmpeg error:\n{}", String::from_utf8_lossy(&output.stderr));
            panic!("ffmpeg error")
        }
    }
}
