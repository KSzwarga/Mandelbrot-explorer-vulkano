mod swapchain;
mod render_pass;
mod pipeline;
mod palettes;

use swapchain::SwapchainBundle;
use render_pass::{create_render_pass, create_framebuffers};
use pipeline::{create_pipeline, PushConstants};
use palettes::PaletteMode;

use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassBeginInfo,
    SubpassContents, SubpassEndInfo,
};

use vulkano::pipeline::Pipeline;
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, DeviceFeatures, Queue, QueueCreateInfo};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::render_pass::{Framebuffer, RenderPass};
use vulkano::swapchain::{acquire_next_image, Surface, SwapchainPresentInfo};
use vulkano::sync::{self, GpuFuture};
use vulkano::Validated;
use vulkano::VulkanLibrary;

use winit::application::ApplicationHandler as WinitApp;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey, ModifiersState};
use winit::window::{Window, WindowId};

use std::sync::Arc;

// ─────────────────────────────────────────────────────────────────────────────
// InputState
// ─────────────────────────────────────────────────────────────────────────────
struct InputState {
    cursor_pos:        [f64; 2],
    drag_start_screen: Option<[f64; 2]>,
    drag_start_center: [f64; 2],
    modifiers:         ModifiersState,
}

impl InputState {
    fn new() -> Self {
        Self {
            cursor_pos:        [0.0, 0.0],
            drag_start_screen: None,
            drag_start_center: [0.0, 0.0],
            modifiers:         ModifiersState::empty(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RenderContext
// ─────────────────────────────────────────────────────────────────────────────
struct RenderContext {
    device:             Arc<Device>,
    queue:              Arc<Queue>,
    swapchain:          SwapchainBundle,
    render_pass:        Arc<RenderPass>,
    framebuffers:       Vec<Arc<Framebuffer>>,
    pipeline:           Arc<GraphicsPipeline>,
    cmd_allocator:      Arc<StandardCommandBufferAllocator>,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    viewport:           Viewport,
    push_constants:     PushConstants,
    center_f64:         [f64; 2],
    zoom_f64:           f64,
    palette:            PaletteMode,
}

impl RenderContext {
    fn rebuild_framebuffers(&mut self) {
        self.framebuffers = create_framebuffers(
            self.render_pass.clone(),
            &self.swapchain.images,
        );
    }

    fn rebuild_viewport(&mut self, width: f32, height: f32) {
        self.viewport = Viewport {
            offset:      [0.0, 0.0],
            extent:      [width, height],
            depth_range: 0.0..=1.0,
        };
        self.push_constants.aspect = width as f64 / height as f64;
    }

    fn sync_push_constants(&mut self) {
        self.push_constants.center = self.center_f64;
        self.push_constants.zoom   = self.zoom_f64;
    }

    // ── Switch palette ────────────────────────────────────────────────────────
    // Rebuilds the Vulkan pipeline with the new fragment shader.
    // Same cost as a window resize — fast enough for interactive switching.
    fn set_palette(&mut self, palette: PaletteMode) {
        self.palette  = palette;
        self.pipeline = create_pipeline(
            self.device.clone(),
            self.render_pass.clone(),
            palette,
        );
        println!("Palette: {}", palette.name());
    }

    fn draw(&mut self, window: &Window) {
        let dims = window.inner_size();
        if dims.width == 0 || dims.height == 0 { return; }

        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        let (image_index, suboptimal, acquire_future) =
            match acquire_next_image(self.swapchain.swapchain.clone(), None) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("acquire_next_image: {e:?} — recreating");
                    self.swapchain = self.swapchain.recreate([dims.width, dims.height]);
                    self.rebuild_framebuffers();
                    self.rebuild_viewport(dims.width as f32, dims.height as f32);
                    return;
                }
            };

        if suboptimal {
            self.swapchain = self.swapchain.recreate([dims.width, dims.height]);
            self.rebuild_framebuffers();
            self.rebuild_viewport(dims.width as f32, dims.height as f32);
        }

        let mut builder = AutoCommandBufferBuilder::primary(
            self.cmd_allocator.clone(),
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        ).unwrap();

        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.0, 0.0, 0.0, 1.0].into())],
                    ..RenderPassBeginInfo::framebuffer(
                        self.framebuffers[image_index as usize].clone(),
                    )
                },
                SubpassBeginInfo { contents: SubpassContents::Inline, ..Default::default() },
            ).unwrap()
            .bind_pipeline_graphics(self.pipeline.clone()).unwrap()
            .set_viewport(0, [self.viewport.clone()].into_iter().collect()).unwrap()
            .push_constants(self.pipeline.layout().clone(), 0, self.push_constants).unwrap();

        unsafe { builder.draw(3, 1, 0, 0).unwrap(); }
        builder.end_render_pass(SubpassEndInfo::default()).unwrap();
        let command_buffer = builder.build().unwrap();

        let future = self.previous_frame_end.take().unwrap()
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer).unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(
                    self.swapchain.swapchain.clone(), image_index,
                ),
            )
            .then_signal_fence_and_flush();

        match future.map_err(Validated::unwrap) {
            Ok(f)  => { self.previous_frame_end = Some(f.boxed()); }
            Err(e) => {
                eprintln!("flush error: {e:?}");
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// App
// ─────────────────────────────────────────────────────────────────────────────
struct App {
    instance: Arc<Instance>,
    window:   Option<Arc<Window>>,
    ctx:      Option<RenderContext>,
    input:    InputState,
}

impl WinitApp<()> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::Window::default_attributes()
                        .with_title(
                            "Mandelbrot — drag/scroll to navigate | R reset | \
                             Tab/Shift+Tab or 1-5 to switch palette"
                        )
                        .with_inner_size(winit::dpi::LogicalSize::new(1280u32, 720u32)),
                )
                .unwrap(),
        );

        let surface = Surface::from_window(self.instance.clone(), window.clone())
            .expect("Failed to create surface");

        // ── Physical device selection ─────────────────────────────────────────
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        let (physical_device, queue_family_index) = self.instance
            .enumerate_physical_devices()
            .unwrap()
            .filter(|p| p.supported_extensions().contains(&device_extensions))
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        q.queue_flags.intersects(vulkano::device::QueueFlags::GRAPHICS)
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu   => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu    => 2,
                PhysicalDeviceType::Cpu           => 3,
                _                                 => 4,
            })
            .expect("No suitable GPU found");

        println!("GPU: {}", physical_device.properties().device_name);

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: device_extensions,
                enabled_features: DeviceFeatures {
                    shader_float64: true,
                    ..DeviceFeatures::empty()
                },
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                ..Default::default()
            },
        ).expect("Failed to create logical device (shaderFloat64 may not be supported)");

        let queue = queues.next().unwrap();

        let initial_palette = PaletteMode::WarmViolet;

        let swapchain    = SwapchainBundle::create(device.clone(), surface.clone(), &window);
        let render_pass  = create_render_pass(device.clone(), swapchain.swapchain.image_format());
        let framebuffers = create_framebuffers(render_pass.clone(), &swapchain.images);
        let pipeline     = create_pipeline(device.clone(), render_pass.clone(), initial_palette);

        let cmd_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(), Default::default(),
        ));

        let window_size = window.inner_size();
        let width  = window_size.width  as f32;
        let height = window_size.height as f32;

        let viewport = Viewport {
            offset:      [0.0, 0.0],
            extent:      [width, height],
            depth_range: 0.0..=1.0,
        };

        let center_f64 = [-0.5_f64, 0.0_f64];
        let zoom_f64   = 3.5_f64;

        let push_constants = PushConstants {
            center: center_f64,
            zoom:   zoom_f64,
            aspect: width as f64 / height as f64,
        };

        let previous_frame_end = Some(sync::now(device.clone()).boxed());

        println!("Ready — Tab/Shift+Tab or 1-5 to cycle palettes. Current: {}", initial_palette.name());

        self.window = Some(window);
        self.ctx    = Some(RenderContext {
            device,
            queue,
            swapchain,
            render_pass,
            framebuffers,
            pipeline,
            cmd_allocator,
            previous_frame_end,
            viewport,
            push_constants,
            center_f64,
            zoom_f64,
            palette: initial_palette,
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::ModifiersChanged(mods) => {
                self.input.modifiers = mods.state();
            }

            WindowEvent::Resized(new_size) => {
                if new_size.width == 0 || new_size.height == 0 { return; }
                if let Some(ctx) = &mut self.ctx {
                    ctx.swapchain = ctx.swapchain.recreate([new_size.width, new_size.height]);
                    ctx.rebuild_framebuffers();
                    ctx.rebuild_viewport(new_size.width as f32, new_size.height as f32);
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                let new_pos = [position.x, position.y];
                if let (Some(ctx), Some(start)) = (&mut self.ctx, self.input.drag_start_screen) {
                    let dx = new_pos[0] - start[0];
                    let dy = new_pos[1] - start[1];
                    let w  = ctx.viewport.extent[0] as f64;
                    let h  = ctx.viewport.extent[1] as f64;
                    let scale = ctx.zoom_f64 / w;
                    ctx.center_f64[0] = self.input.drag_start_center[0] - dx * scale * (w / h);
                    ctx.center_f64[1] = self.input.drag_start_center[1] - dy * scale;
                    ctx.sync_push_constants();
                }
                self.input.cursor_pos = new_pos;
            }

            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                if let Some(ctx) = &mut self.ctx {
                    match state {
                        ElementState::Pressed => {
                            self.input.drag_start_screen = Some(self.input.cursor_pos);
                            self.input.drag_start_center = ctx.center_f64;
                        }
                        ElementState::Released => {
                            self.input.drag_start_screen = None;
                        }
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(ctx) = &mut self.ctx {
                    let lines = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y as f64,
                        MouseScrollDelta::PixelDelta(pos) => pos.y / 40.0,
                    };
                    let factor = 0.9_f64.powf(lines);
                    let w = ctx.viewport.extent[0] as f64;
                    let h = ctx.viewport.extent[1] as f64;
                    let u = self.input.cursor_pos[0] / w;
                    let v = self.input.cursor_pos[1] / h;
                    let mouse_re = ctx.center_f64[0] + (u - 0.5) * ctx.zoom_f64 * (w / h);
                    let mouse_im = ctx.center_f64[1] + (v - 0.5) * ctx.zoom_f64;
                    ctx.zoom_f64 *= factor;
                    ctx.zoom_f64  = ctx.zoom_f64.clamp(1e-14, 4.0);
                    ctx.center_f64[0] = mouse_re - (u - 0.5) * ctx.zoom_f64 * (w / h);
                    ctx.center_f64[1] = mouse_im - (v - 0.5) * ctx.zoom_f64;
                    ctx.sync_push_constants();
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    if let PhysicalKey::Code(key) = event.physical_key {
                        if let Some(ctx) = &mut self.ctx {
                            match key {
                                // ── View controls ─────────────────────────
                                KeyCode::KeyR => {
                                    ctx.center_f64 = [-0.5, 0.0];
                                    ctx.zoom_f64   = 3.5;
                                    ctx.sync_push_constants();
                                    println!("View reset");
                                }

                                // ── Palette cycling ───────────────────────
                                // Tab → next palette
                                // Shift+Tab → previous palette
                                KeyCode::Tab => {
                                    let next = if self.input.modifiers.shift_key() {
                                        ctx.palette.prev()
                                    } else {
                                        ctx.palette.next()
                                    };
                                    ctx.set_palette(next);
                                }

                                // Direct selection: 1–5
                                KeyCode::Digit1 => ctx.set_palette(PaletteMode::WarmViolet),
                                KeyCode::Digit2 => ctx.set_palette(PaletteMode::BlueFire),
                                KeyCode::Digit3 => ctx.set_palette(PaletteMode::Midnight),
                                KeyCode::Digit4 => ctx.set_palette(PaletteMode::Acid),
                                KeyCode::Digit5 => ctx.set_palette(PaletteMode::Greyscale),

                                _ => {}
                            }
                        }
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                if let (Some(ctx), Some(window)) = (&mut self.ctx, &self.window) {
                    ctx.draw(window);
                    window.request_redraw();
                }
            }

            _ => {}
        }
    }
}

fn main() {
    let library = VulkanLibrary::new().expect(
        "Vulkan not found — is the SDK installed and your driver up to date?",
    );

    let event_loop = EventLoop::new().unwrap();

    let instance = Instance::new(
        library,
        InstanceCreateInfo {
            enabled_extensions: Surface::required_extensions(&event_loop).unwrap(),
            ..InstanceCreateInfo::application_from_cargo_toml()
        },
    ).unwrap();

    println!("Vulkan instance: {:?}", instance.api_version());
    println!("Palettes:");
    for (i, p) in PaletteMode::ALL.iter().enumerate() {
        println!("  {} — {}", i + 1, p.name());
    }

    let mut app = App {
        instance: instance.into(),
        window:   None,
        ctx:      None,
        input:    InputState::new(),
    };

    event_loop.run_app(&mut app).unwrap();
}
