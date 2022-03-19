use std::ffi::OsString;
use std::num::NonZeroU32;
use std::path::Path;
use clap::Parser;
use winit::dpi::PhysicalSize;
use vivotk::render::wgpu::camera::{Camera, CameraState};
use vivotk::render::wgpu::reader::{PcdFileReader, RenderReader};
use vivotk::render::wgpu::renderer::PointCloudRenderer;

/// Converts a folder of .pcd files to a folder of .png images
#[derive(Parser)]
struct Args {
    /// Directory with all the pcd files in lexicographical order
    #[clap(long)]
    pcds: OsString,
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

fn main() {
    pollster::block_on(run());
}

async fn run() {
    let args: Args = Args::parse();
    let path = Path::new(&args.pcds);
    let reader = PcdFileReader::from_directory(path);

    if reader.len() == 0 {
        eprintln!("Must provide at least one file!");
        return;
    }

    let output_path = Path::new(&args.output_dir);
    std::fs::create_dir_all(output_path).expect("Failed to create output directory");

    let frames = if let Some(frames) = args.frames {
        frames.min(reader.len())
    } else {
        reader.len()
    };

    let size = PhysicalSize::new(args.width, args.height);

    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .unwrap();
    let (device, queue) = adapter
        .request_device(&Default::default(), None)
        .await
        .unwrap();

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
        usage: wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::MAP_READ,
        label: None,
        mapped_at_creation: false,
    };
    let output_buffer = device.create_buffer(&output_buffer_desc);

    let camera = Camera::new((args.camera_x, args.camera_y, args.camera_z), cgmath::Deg(args.camera_yaw), cgmath::Deg(args.camera_pitch));
    let camera_state = CameraState::new(camera, size.width, size.height);
    let mut point_renderer = PointCloudRenderer::new(&device, texture_desc.format, reader.get_at(0).unwrap(), size, &camera_state);

    for i in 0..frames {
        point_renderer.update_vertices(&device, &queue, reader.get_at(i).unwrap());
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        point_renderer.render(&mut encoder, &texture_view);
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: &output_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: NonZeroU32::new(u32_size * size.width),
                    rows_per_image: NonZeroU32::new(size.height),
                },
            },
            texture_desc.size,
        );

        queue.submit(Some(encoder.finish()));
        {
            let buffer_slice = output_buffer.slice(..);
            let mapping = buffer_slice.map_async(wgpu::MapMode::Read);
            device.poll(wgpu::Maintain::Wait);
            mapping.await.unwrap();

            let data = buffer_slice.get_mapped_range();

            use image::{ImageBuffer, Rgba};
            let buffer =
                ImageBuffer::<Rgba<u8>, _>::from_raw(size.width, size.height, data).unwrap();


            let default_name = format!("{i}");
            let filename = reader.file_at(i).expect("Should have file")
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
                .or(Some(default_name))
                .unwrap();

            let filename = format!("{filename}.png");
            buffer.save(
                output_path.join(Path::new(&filename))).unwrap();
        }
        output_buffer.unmap();
    }

}