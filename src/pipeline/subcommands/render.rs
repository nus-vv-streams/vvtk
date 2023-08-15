use super::Subcommand;
use crate::pipeline::channel::Channel;
use crate::pipeline::PipelineMessage;
use crate::render::wgpu::png::{PngWriter, RenderFormat};
use cgmath::num_traits::pow;
use clap::Parser;
use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};

/// Writes point clouds from the input stream into images.
#[derive(Parser)]
pub struct Args {
    /// Directory to store output png images
    output_dir: OsString,
    #[clap(short = 'x', long, default_value_t = 0.0)]
    camera_x: f32,
    #[clap(short = 'y', long, default_value_t = 0.0)]
    camera_y: f32,
    #[clap(short = 'z', long, default_value_t = 1.8)]
    camera_z: f32,
    #[clap(long = "yaw", default_value_t = -90.0, allow_hyphen_values = true)]
    camera_yaw: f32,
    #[clap(long = "pitch", default_value_t = 0.0)]
    camera_pitch: f32,
    #[clap(long, default_value_t = 1600)]
    width: u32,
    #[clap(long, default_value_t = 900)]
    height: u32,
    #[clap(long, default_value_t = 5)]
    name_length: u32,
    #[clap(long, default_value = "rgb(255,255,255)")]
    bg_color: OsString,
    #[clap(long = "format", default_value_t = RenderFormat::Png)]
    render_format: RenderFormat,
    #[clap(long, default_value_t = false)]
    verbose: bool,
    #[clap(long, default_value_t = 30.0)]
    fps: f32,
}

pub struct Render<'a> {
    writer: PngWriter<'a>,
    name_length: u32,
    count: u32,
    verbose: bool,
    fps: f32,
}

impl<'a> Render<'a> {
    pub fn from_args(args: Vec<String>) -> Box<dyn Subcommand> {
        let Args {
            output_dir,
            camera_x,
            camera_y,
            camera_z,
            camera_yaw,
            camera_pitch,
            width,
            height,
            name_length,
            bg_color,
            render_format,
            verbose,
            fps,
        }: Args = Args::parse_from(args);

        let mut output_dir = output_dir;
        if render_format == RenderFormat::Mp4 {
            // check ffmpeg existence first
            let _ffmpeg_check = Command::new("ffmpeg")
                .arg("-version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("Failed to check ffmpeg existence. Please install ffmpeg first.");

            output_dir = (output_dir.into_string().unwrap() + "/.tmp_mp4").into();
            // create the directory if it doesn't exist
            // if exists, check if it's empty
            // if not empty, panic

            let checked_path = Path::new(&output_dir);
            if !checked_path.exists() {
                std::fs::create_dir_all(checked_path).expect("Failed to create output directory");
            } else {
                if checked_path.read_dir().unwrap().next().is_some() {
                    panic!(
                        "Temp png directory({}) is not empty, please backup and remove the files",
                        output_dir.to_str().unwrap()
                    )
                }
                if checked_path.is_file() {
                    panic!(
                        "Temp png directory({}) is a file, please rename this file",
                        output_dir.to_str().unwrap()
                    );
                }
            }
        }

        Box::from(Render {
            writer: PngWriter::new(
                output_dir,
                camera_x,
                camera_y,
                camera_z,
                camera_yaw,
                camera_pitch,
                width,
                height,
                bg_color.to_str().unwrap(),
                render_format,
            ),
            name_length,
            count: 0,
            verbose,
            fps,
        })
    }
}

impl Subcommand for Render<'_> {
    fn handle(&mut self, messages: Vec<PipelineMessage>, channel: &Channel) {
        let max_count = pow(10, self.name_length as usize);

        for message in messages {
            match &message {
                PipelineMessage::IndexedPointCloud(pc, i) => {
                    let padded_count = format!("{:0>width$}", i, width = self.name_length as usize);
                    let filename = format!("{}.png", padded_count);
                    self.count += 1;
                    if self.count >= max_count {
                        channel.send(PipelineMessage::End);
                        panic!("Too many files, please increase the name length by setting --name-length")
                    }
                    self.writer.write_to_png(pc, &filename);
                }
                _ => {}
            }
            channel.send(message);
        }
    }
}

impl Drop for Render<'_> {
    fn drop(&mut self) {
        if self.writer.render_format() == RenderFormat::Mp4 {
            self.writer
                .write_to_mp4(self.name_length, self.fps, self.verbose);
        }
        // drop writer
        // drop(&self.writer);
    }
}

// pub fn pc_to_png(to_png: &mut ToPng, pc: PointCloud<PointXyzRgba>, filename: &str) {
//     if to_png.point_renderer.is_none() {
//         to_png.point_renderer = Some(PointCloudRenderer::new(
//             &to_png.device,
//             to_png.texture_desc.format,
//             &pc,
//             to_png.size,
//             &to_png.camera_state,
//         ));
//     }
//     let point_renderer = to_png.point_renderer.as_mut().unwrap();
//     point_renderer.update_vertices(&to_png.device, &to_png.queue, &pc);
//     let mut encoder = to_png
//         .device
//         .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

//     point_renderer.render(&mut encoder, &to_png.texture_view);
//     encoder.copy_texture_to_buffer(
//         wgpu::ImageCopyTexture {
//             aspect: wgpu::TextureAspect::All,
//             texture: &to_png.texture,
//             mip_level: 0,
//             origin: wgpu::Origin3d::ZERO,
//         },
//         wgpu::ImageCopyBuffer {
//             buffer: &to_png.output_buffer,
//             layout: wgpu::ImageDataLayout {
//                 offset: 0,
//                 bytes_per_row: NonZeroU32::new(to_png.u32_size * to_png.size.width),
//                 rows_per_image: NonZeroU32::new(to_png.size.height),
//             },
//         },
//         to_png.texture_desc.size,
//     );

//     to_png.queue.submit(Some(encoder.finish()));
//     {
//         let buffer_slice = to_png.output_buffer.slice(..);
//         buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
//         to_png.device.poll(wgpu::Maintain::Wait);

//         let data = buffer_slice.get_mapped_range();

//         use image::{ImageBuffer, Rgba};
//         let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(
//             to_png.size.width,
//             to_png.size.height,
//             data,
//         )
//         .unwrap();

//         let filename = format!("{}.png", filename);
//         to_png.count += 1;
//         let output_path = Path::new(&to_png.output_dir);
//         buffer.save(output_path.join(Path::new(&filename))).unwrap();
//     }
//     to_png.output_buffer.unmap();

// }
