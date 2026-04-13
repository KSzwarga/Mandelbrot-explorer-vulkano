use std::sync::Arc;

use vulkano::image::Image;
use vulkano::device::Device;
use vulkano::swapchain::{
    ColorSpace, CompositeAlpha, PresentMode, Surface, SurfaceInfo, Swapchain,
    SwapchainCreateInfo,
};
use winit::window::Window;


pub struct SwapchainBundle {
    pub swapchain: Arc<Swapchain>,
    pub images:    Vec<Arc<Image>>,
}

impl SwapchainBundle {
    // ─────────────────────────────────────────────────────────────────────────
    // create  —  called once from resumed()
    //
    // Queries the surface for its capabilities and chooses:
    //   • image count   : as close to 3 (triple-buffer) as the GPU allows
    //   • format        : prefers B8G8R8A8_SRGB, falls back to first supported
    //   • present mode  : prefers Mailbox (low-latency), falls back to Fifo
    //   • extent        : matches current window inner size
    //   • composite     : Opaque (no window transparency needed)
    // ─────────────────────────────────────────────────────────────────────────
    pub fn create(
        device:  Arc<Device>,
        surface: Arc<Surface>,
        window:  &Window,
    ) -> Self {
        // Surface capabilities tell us the constraints we must stay within.
        let caps = device
            .physical_device()
            .surface_capabilities(&surface, SurfaceInfo::default())
            .expect("Failed to query surface capabilities");

        // ── Image count ───────────────────────────────────────────────────────
        // We want 3 images for triple-buffering (smooth rendering without
        // waiting for the GPU). Clamp to what the surface allows.
        let image_count = {
            let desired = 3;
            let min     = caps.min_image_count;
            // max_image_count of 0 means "no upper limit"
            let max     = caps.max_image_count.unwrap_or(desired);
            desired.clamp(min, max)
        };


        let surface_formats = device
            .physical_device()
            .surface_formats(&surface, SurfaceInfo::default())
            .expect("Failed to query surface formats");

        let (image_format, image_color_space) = surface_formats
            .iter()
            .find(|(fmt, cs)| {
                *fmt == vulkano::format::Format::B8G8R8A8_SRGB
                    && *cs == ColorSpace::SrgbNonLinear
            })
            // Fall back to the first format the surface reports
            .copied()
            .unwrap_or(surface_formats[0]);


        let present_modes = device
            .physical_device()
            .surface_present_modes(&surface, SurfaceInfo::default())
            .expect("Failed to query present modes");

        let present_mode = present_modes
            .into_iter()
            .find(|&m| m == PresentMode::Mailbox)
            .unwrap_or(PresentMode::Fifo);

        let window_size = window.inner_size();
        let image_extent = [window_size.width, window_size.height];

        println!(
            "Swapchain: {}×{} | format {:?} | mode {:?} | {} images",
            image_extent[0], image_extent[1],
            image_format, present_mode, image_count
        );

        // ── Create ────────────────────────────────────────────────────────────
        let (swapchain, images) = Swapchain::new(
            device,
            surface,
            SwapchainCreateInfo {
                min_image_count:    image_count,
                image_format,
                image_color_space,
                image_extent,
                image_usage:        vulkano::image::ImageUsage::COLOR_ATTACHMENT,
                composite_alpha:    CompositeAlpha::Opaque,
                present_mode,
                ..Default::default()
            },
        )
        .expect("Failed to create swapchain");

        Self { swapchain, images }
    }


    pub fn recreate(&self, new_size: [u32; 2]) -> Self {
        let (swapchain, images) = self
            .swapchain
            .recreate(SwapchainCreateInfo {
                image_extent: new_size,
                ..self.swapchain.create_info()
            })
            .expect("Failed to recreate swapchain");

        Self { swapchain, images }
    }
}
