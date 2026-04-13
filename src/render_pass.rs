use std::sync::Arc;

use vulkano::device::Device;
use vulkano::format::Format;
use vulkano::image::view::ImageView;   // fixed: removed stray `self`
use vulkano::image::Image;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};

// ─────────────────────────────────────────────────────────────────────────────
// create_render_pass
//
// A render pass tells Vulkan the *shape* of one rendering operation before
// any commands are recorded:
//
//   • What image formats will be written to         (attachments)
//   • What to do at the start of the pass           (load op)
//   • What to do at the end                         (store op)
//   • What layout the image must be in at each step (image layouts)

// ─────────────────────────────────────────────────────────────────────────────
pub fn create_render_pass(
    device:           Arc<Device>,
    swapchain_format: Format,
) -> Arc<RenderPass> {
    vulkano::single_pass_renderpass!(
        device,
        attachments: {
            color: {
                format: swapchain_format,
                // MSAA: 1 = no multisampling.
                samples: 1,
                load_op: Clear,
                store_op: Store,
            },
        },
        pass: {
            color: [color],
            depth_stencil: {},
        },
    )
    .expect("Failed to create render pass")
}

// ─────────────────────────────────────────────────────────────────────────────
// One framebuffer per swapchain image
// ─────────────────────────────────────────────────────────────────────────────
pub fn create_framebuffers(
    render_pass: Arc<RenderPass>,
    images:      &[Arc<Image>],
) -> Vec<Arc<Framebuffer>> {
    images
        .iter()
        .map(|image| {
            // ImageView describes how to interpret the raw image data.
            // Framebuffers always attach via views, never raw images.
            let view = ImageView::new_default(image.clone())
                .expect("Failed to create image view");

            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    // Order must match the render pass attachment list: [color].
                    attachments: vec![view],
                    ..Default::default()
                },
            )
            .expect("Failed to create framebuffer")
        })
        .collect()
}
