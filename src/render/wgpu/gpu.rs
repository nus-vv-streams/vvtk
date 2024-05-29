use wgpu::InstanceDescriptor;
use winit::window::Window;

pub struct WindowGpu {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub adapter: wgpu::Adapter,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
}

impl WindowGpu {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(InstanceDescriptor::default());

        // The surface is the part of the window that we draw to.
        //
        // Safety!: The surface needs to live as long as the window that created it.
        let surface = unsafe { instance.create_surface(window).unwrap() };
        // The adapter is a handle to our actual graphics card.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    #[cfg(not(feature = "fullscreen"))]
                    limits: wgpu::Limits::downlevel_defaults(),
                    // https://github.com/gfx-rs/wgpu/discussions/2952?sort=top
                    #[cfg(feature = "fullscreen")]
                    limits: adapter.limits(),
                },
                None, // Trace path
            )
            .await
            .unwrap();

        // config defines how the surface creates its underlying `SurfaceTexture`
        let surface_caps = surface.get_capabilities(&adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            // wgpu::TextureFormat::Bgra8UnormSrgb,
            format: surface_caps.formats[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![], // new  0.15
        };
        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            adapter,
            config,
            size,
        }
    }

    /// Reconfigure the surface every time the window's size changes
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            // print!("WindowGpu::resize {:#?}\n", new_size);
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Returns a new `SurfaceTexture` that we will render to and a `TextureView` with default settings
    pub fn create_view(
        &self,
    ) -> Result<(wgpu::SurfaceTexture, wgpu::TextureView), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        Ok((output, view))
    }

    /// Creates a command encoder that will record commands to send to the gpu in a command buffer.
    /// Call `encoder.finish()` to get the CommandBuffer.
    pub fn create_encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            })
    }
}
