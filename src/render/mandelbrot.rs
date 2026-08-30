//! The fractal pass: one full-screen triangle, perturbation-rendered.

use crate::render::context::Gpu;
use crate::render::pipeline::{self, Resources};
use crate::render::uniforms::GpuUniforms;
use crate::scene::Scene;

/// Background behind the fractal.
const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.02,
    g: 0.02,
    b: 0.06,
    a: 1.0,
};

/// Vertices in the covering triangle
const COVERING_TRIANGLE_VERTICES: u32 = 3;

pub struct FractalRenderer {
    resources: Resources,
    /// Reference generation currently sitting in `orbit_buffer`. `None` until
    /// the first upload
    uploaded_generation: Option<u64>,
}

impl FractalRenderer {
    pub fn new(gpu: &Gpu) -> Self {
        Self {
            resources: pipeline::build(gpu),
            uploaded_generation: None,
        }
    }

    pub fn encode(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        scene: &Scene,
    ) {
        let reference = scene.reference();

        // Uniforms are 32 bytes
        let uniforms = GpuUniforms::from_view(
            &scene.view,
            reference,
            [gpu.config.width, gpu.config.height],
            scene.palette,
        );
        gpu.queue.write_buffer(
            &self.resources.uniform_buffer,
            0,
            bytemuck::bytes_of(&uniforms),
        );

        // The orbit up to 800 KB.
        if self.uploaded_generation != Some(scene.reference_generation()) {
            gpu.queue.write_buffer(
                &self.resources.orbit_buffer,
                0,
                bytemuck::cast_slice(&reference.orbit),
            );
            self.uploaded_generation = Some(scene.reference_generation());
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fractal pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        pass.set_pipeline(&self.resources.pipeline);
        pass.set_bind_group(0, &self.resources.bind_group, &[]);
        pass.draw(0..COVERING_TRIANGLE_VERTICES, 0..1);
    }
}
