use std::sync::Arc;

use bytemuck::{Pod, Zeroable};

use vulkano::device::Device;
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::VertexInputState;
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::pipeline::DynamicState;
use vulkano::render_pass::{RenderPass, Subpass};

use crate::palettes::PaletteMode;

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
#version 460
layout(location = 0) out vec2 uv;
void main() {
    vec2 pos = vec2(
        (gl_VertexIndex == 1) ? 3.0 : -1.0,
        (gl_VertexIndex == 2) ? 3.0 : -1.0
    );
    gl_Position = vec4(pos, 0.0, 1.0);
    uv = pos * 0.5 + 0.5;
}
        "
    }
}

mod fs_warm_violet {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
#version 460
#extension GL_ARB_gpu_shader_fp64 : require
layout(location = 0) in  vec2 uv;
layout(location = 0) out vec4 f_color;
layout(push_constant) uniform PushConstants { dvec2 center; double zoom; double aspect; } pc;
const int MAX_ITER = 5000; const double BAILOUT = 256.0LF; const int SAMPLES = 4;
const vec2 JITTER[4] = vec2[4](vec2(1,3),vec2(-1,-3),vec2(3,-1),vec2(-3,1));
vec4 palette(int iter, double len2) {
    if (iter == MAX_ITER) return vec4(0,0,0,1);
    float si = float(iter)+1.0-log2(log2(sqrt(float(len2)))/log2(float(BAILOUT)));
    float t = log(si+1.0)/log(float(MAX_ITER)+1.0);
    vec3 col = vec3(0.38)+vec3(0.28)*cos(6.28318*(t*12.0+vec3(0.00,0.62,0.45)));
    col = min(col, vec3(0.55));
    return 0.8*vec4(col,1);
}
vec4 ms(dvec2 uv64) {
    dvec2 c=(uv64-0.5LF)*dvec2(pc.aspect,1.0LF)*pc.zoom+pc.center;
    {double q=(c.x-0.25LF)*(c.x-0.25LF)+c.y*c.y; if(q*(q+(c.x-0.25LF))<0.25LF*c.y*c.y) return vec4(0,0,0,1);}
    {double dx=c.x+1.0LF; if(dx*dx+c.y*c.y<0.0625LF) return vec4(0,0,0,1);}
    dvec2 z=dvec2(0.0LF); int iter=0; double len2=0.0LF;
    for(int i=0;i<MAX_ITER;i++){len2=dot(z,z);if(len2>BAILOUT)break;z=dvec2(z.x*z.x-z.y*z.y,2.0LF*z.x*z.y)+c;iter++;}
    return palette(iter,len2);
}
void main() {
    vec2 dx=dFdx(uv); vec2 dy=dFdy(uv); vec4 a=vec4(0);
    for(int s=0;s<SAMPLES;s++){vec2 jp=vec2(JITTER[s].x,JITTER[s].y)/8.0;a+=ms(dvec2(uv+dx*jp.x+dy*jp.y));}
    f_color=a/float(SAMPLES);
}
        "
    }
}

mod fs_blue_fire {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
#version 460
#extension GL_ARB_gpu_shader_fp64 : require
layout(location = 0) in  vec2 uv;
layout(location = 0) out vec4 f_color;
layout(push_constant) uniform PushConstants { dvec2 center; double zoom; double aspect; } pc;
const int MAX_ITER = 5000; const double BAILOUT = 256.0LF; const int SAMPLES = 4;
const vec2 JITTER[4] = vec2[4](vec2(1,3),vec2(-1,-3),vec2(3,-1),vec2(-3,1));
vec4 palette(int iter, double len2) {
    if (iter == MAX_ITER) return vec4(0,0,0,1);
    float si = float(iter)+1.0-log2(log2(sqrt(float(len2)))/log2(float(BAILOUT)));
    float t = log(si+1.0)/log(float(MAX_ITER)+1.0);
    vec3 col = vec3(0.5)+vec3(0.5)*cos(6.28318*(t*8.0+vec3(0.0,0.10,0.55)));
    return vec4(col,1);
}
vec4 ms(dvec2 uv64) {
    dvec2 c=(uv64-0.5LF)*dvec2(pc.aspect,1.0LF)*pc.zoom+pc.center;
    {double q=(c.x-0.25LF)*(c.x-0.25LF)+c.y*c.y; if(q*(q+(c.x-0.25LF))<0.25LF*c.y*c.y) return vec4(0,0,0,1);}
    {double dx=c.x+1.0LF; if(dx*dx+c.y*c.y<0.0625LF) return vec4(0,0,0,1);}
    dvec2 z=dvec2(0.0LF); int iter=0; double len2=0.0LF;
    for(int i=0;i<MAX_ITER;i++){len2=dot(z,z);if(len2>BAILOUT)break;z=dvec2(z.x*z.x-z.y*z.y,2.0LF*z.x*z.y)+c;iter++;}
    return palette(iter,len2);
}
void main() {
    vec2 dx=dFdx(uv); vec2 dy=dFdy(uv); vec4 a=vec4(0);
    for(int s=0;s<SAMPLES;s++){vec2 jp=vec2(JITTER[s].x,JITTER[s].y)/8.0;a+=ms(dvec2(uv+dx*jp.x+dy*jp.y));}
    f_color=a/float(SAMPLES);
}
        "
    }
}

mod fs_midnight {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
#version 460
#extension GL_ARB_gpu_shader_fp64 : require
layout(location = 0) in  vec2 uv;
layout(location = 0) out vec4 f_color;
layout(push_constant) uniform PushConstants { dvec2 center; double zoom; double aspect; } pc;
const int MAX_ITER = 5000; const double BAILOUT = 256.0LF; const int SAMPLES = 1;
const vec2 JITTER[4] = vec2[4](vec2(1,3),vec2(-1,-3),vec2(3,-1),vec2(-3,1));
vec4 palette(int iter, double len2) {
    if (iter == MAX_ITER) return vec4(0,0,0,1);
    float si = float(iter)+1.0-log2(log2(sqrt(float(len2)))/log2(float(BAILOUT)));
    float t = log(si+1.0)/log(float(MAX_ITER)+1.0);
    float glow = pow(1.0-t, 4.0);
    vec3 electric = vec3(0.05,0.3,1.0);
    vec3 gold     = vec3(1.0,0.7,0.1);
    vec3 col = mix(electric, gold, pow(1.0-t, 12.0)) * glow;
    return vec4(col,1);
}
vec4 ms(dvec2 uv64) {
    dvec2 c=(uv64-0.5LF)*dvec2(pc.aspect,1.0LF)*pc.zoom+pc.center;
    {double q=(c.x-0.25LF)*(c.x-0.25LF)+c.y*c.y; if(q*(q+(c.x-0.25LF))<0.25LF*c.y*c.y) return vec4(0,0,0,1);}
    {double dx=c.x+1.0LF; if(dx*dx+c.y*c.y<0.0625LF) return vec4(0,0,0,1);}
    dvec2 z=dvec2(0.0LF); int iter=0; double len2=0.0LF;
    for(int i=0;i<MAX_ITER;i++){len2=dot(z,z);if(len2>BAILOUT)break;z=dvec2(z.x*z.x-z.y*z.y,2.0LF*z.x*z.y)+c;iter++;}
    return palette(iter,len2);
}
void main() {
    vec2 dx=dFdx(uv); vec2 dy=dFdy(uv); vec4 a=vec4(0);
    for(int s=0;s<SAMPLES;s++){vec2 jp=vec2(JITTER[s].x,JITTER[s].y)/8.0;a+=ms(dvec2(uv+dx*jp.x+dy*jp.y));}
    f_color=a;
}
        "
    }
}

mod fs_acid {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
#version 460
#extension GL_ARB_gpu_shader_fp64 : require

layout(location = 0) in  vec2 uv;        // arrives as f32 from rasteriser
layout(location = 0) out vec4 f_color;

// ── Push constants — all f64 ──────────────────────────────────────────────────
// dvec2  = 2 × double = 16 bytes, 16-byte aligned  → offset 0
// double = 8 bytes                                  → offset 16
// double = 8 bytes                                  → offset 24
// Total  = 32 bytes. Must match PushConstants in pipeline.rs exactly.
layout(push_constant) uniform PushConstants {
    dvec2  center;
    double zoom;
    double aspect;
} pc;

const int    MAX_ITER = 5000;
const double BAILOUT  = 1000.0LF;  // LF suffix = double literal in GLSL

void main() {
    // ── Promote UV to f64, map to complex plane ───────────────────────────────
    // uv is vec2 (f32) from the vertex shader. We cast to dvec2 immediately
    // so every subsequent operation is in double precision.
    dvec2 uv64 = dvec2(uv);
    dvec2 c    = (uv64 - 0.5LF) * dvec2(pc.aspect, 1.0LF) * pc.zoom + pc.center;

    // ── Skip main cardioid — all f64 ─────────────────────────────────────────
    // Algebraic test for the large central black region. Saves up to MAX_ITER
    // iterations per pixel in the most-viewed area of the set.
    {
        double q = (c.x - 0.25LF)*(c.x - 0.25LF) + c.y*c.y;
        if (q * (q + (c.x - 0.25LF)) < 0.25LF * c.y * c.y) {
            f_color = vec4(0.0, 0.0, 0.0, 1.0);
            return;
        }
    }

    // ── Skip period-2 bulb — all f64 ─────────────────────────────────────────
    {
        double dx = c.x + 1.0LF;
        if (dx*dx + c.y*c.y < 0.0625LF) {
            f_color = vec4(0.0, 0.0, 0.0, 1.0);
            return;
        }
    }

    // ── Mandelbrot iteration — entirely f64 ───────────────────────────────────
    // z = z² + c,  z₀ = 0
    // Escape when |z|² > BAILOUT.
    dvec2  z    = dvec2(0.0LF);
    int    iter = 0;
    double len2 = 0.0LF;

    for (int i = 0; i < MAX_ITER; i++) {
        len2 = dot(z, z);
        if (len2 > BAILOUT) break;
        // z² in component form:
        //   Re(z²) = Re(z)² − Im(z)²
        //   Im(z²) = 2·Re(z)·Im(z)
        z = dvec2(z.x*z.x - z.y*z.y, 2.0LF*z.x*z.y) + c;
        iter++;
    }

    // ── Colouring — drop back to f32 for the palette ──────────────────────────
    // The cosine palette only needs the iteration count (an int) and |z|
    // at escape — both are cast to float safely. Colour doesn't need f64.
    if (iter == MAX_ITER) {
        f_color = vec4(0.0, 0.0, 0.0, 1.0);
    } else {
        // Smooth (continuous) iteration count eliminates colour banding.
        float quotient = float(iter) / float(MAX_ITER);
        float color = clamp(quotient, 0.0, 2.0);

        vec3 col;

        if (quotient > 0.5) {
            // green → white
            col = vec3(color, 1.0, color);
        } else {
            // black → green
            col = vec3(0.0, color, 0.0);
        }
        f_color = vec4(col, 1.0);
    }
}
        "
    }
}

mod fs_greyscale {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
#version 460
#extension GL_ARB_gpu_shader_fp64 : require
layout(location = 0) in  vec2 uv;
layout(location = 0) out vec4 f_color;
layout(push_constant) uniform PushConstants { dvec2 center; double zoom; double aspect; } pc;
const int MAX_ITER = 5000; const double BAILOUT = 256.0LF; const int SAMPLES = 4;
const vec2 JITTER[4] = vec2[4](vec2(1,3),vec2(-1,-3),vec2(3,-1),vec2(-3,1));
vec4 palette(int iter, double len2) {
    if (iter == MAX_ITER) return vec4(0,0,0,1);
    float si = float(iter)+1.0-log2(log2(sqrt(float(len2)))/log2(float(BAILOUT)));
    float t = log(si+1.0)/log(float(MAX_ITER)+1.0);
    float v = 0.5+0.5*sin(t*40.0*3.14159);
    v = pow(v, 0.4);
    return vec4(vec3(v),1);
}
vec4 ms(dvec2 uv64) {
    dvec2 c=(uv64-0.5LF)*dvec2(pc.aspect,1.0LF)*pc.zoom+pc.center;
    {double q=(c.x-0.25LF)*(c.x-0.25LF)+c.y*c.y; if(q*(q+(c.x-0.25LF))<0.25LF*c.y*c.y) return vec4(0,0,0,1);}
    {double dx=c.x+1.0LF; if(dx*dx+c.y*c.y<0.0625LF) return vec4(0,0,0,1);}
    dvec2 z=dvec2(0.0LF); int iter=0; double len2=0.0LF;
    for(int i=0;i<MAX_ITER;i++){len2=dot(z,z);if(len2>BAILOUT)break;z=dvec2(z.x*z.x-z.y*z.y,2.0LF*z.x*z.y)+c;iter++;}
    return palette(iter,len2);
}
void main() {
    vec2 dx=dFdx(uv); vec2 dy=dFdy(uv); vec4 a=vec4(0);
    for(int s=0;s<SAMPLES;s++){vec2 jp=vec2(JITTER[s].x,JITTER[s].y)/8.0;a+=ms(dvec2(uv+dx*jp.x+dy*jp.y));}
    f_color=a/float(SAMPLES);
}
        "
    }
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct PushConstants {
    pub center: [f64; 2],
    pub zoom:   f64,
    pub aspect: f64,
}

pub fn create_pipeline(
    device:      Arc<Device>,
    render_pass: Arc<RenderPass>,
    palette:     PaletteMode,
) -> Arc<GraphicsPipeline> {
    let vs = vs::load(device.clone()).expect("Failed to load vertex shader");

    let fs = match palette {
        PaletteMode::WarmViolet => fs_warm_violet::load(device.clone()),
        PaletteMode::BlueFire   => fs_blue_fire::load(device.clone()),
        PaletteMode::Midnight   => fs_midnight::load(device.clone()),
        PaletteMode::Acid     => fs_acid::load(device.clone()),
        PaletteMode::Greyscale  => fs_greyscale::load(device.clone()),
    }
    .expect("Failed to load fragment shader");

    let vs_entry = vs.entry_point("main").unwrap();
    let fs_entry = fs.entry_point("main").unwrap();

    let subpass = Subpass::from(render_pass.clone(), 0)
        .expect("Subpass 0 not found in render pass");

    let stages = [
        PipelineShaderStageCreateInfo::new(vs_entry),
        PipelineShaderStageCreateInfo::new(fs_entry),
    ];

    let layout = PipelineLayout::new(
        device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
            .into_pipeline_layout_create_info(device.clone())
            .expect("Failed to create pipeline layout info"),
    )
    .expect("Failed to create pipeline layout");

    GraphicsPipeline::new(
        device.clone(),
        None,
        GraphicsPipelineCreateInfo {
            stages:               stages.into_iter().collect(),
            vertex_input_state:   Some(VertexInputState::default()),
            input_assembly_state: Some(InputAssemblyState::default()),
            viewport_state:       Some(ViewportState::default()),
            dynamic_state:        [DynamicState::Viewport].into_iter().collect(),
            rasterization_state:  Some(RasterizationState::default()),
            multisample_state:    Some(MultisampleState::default()),
            color_blend_state:    Some(ColorBlendState::with_attachment_states(
                subpass.num_color_attachments(),
                ColorBlendAttachmentState::default(),
            )),
            subpass: Some(subpass.into()),
            ..GraphicsPipelineCreateInfo::layout(layout)
        },
    )
    .expect("Failed to create graphics pipeline")
}
