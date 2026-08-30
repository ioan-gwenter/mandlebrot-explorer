use crate::fractal::{palette::Palette, reference::Reference, view::View};

/// Mirrors `Uniforms` in `shaders/mandelbrot.wgsl`. Field order and size must stay
/// in lockstep with it; the `min_binding_size` in the bind group layout makes wgpu
/// reject a mismatch at pipeline creation.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuUniforms {
    pub ref_offset: [f32; 2], // view_center - ref_center, ALREADY subtracted in f64
    pub scale: f32,
    pub max_iter: u32,
    pub resolution: [f32; 2],
    pub palette: u32,
    pub ref_len: u32,
}

// Uniform address space requires a size that is a multiple of 16.
const _: () = assert!(std::mem::size_of::<GpuUniforms>() == 32);
const _: () = assert!(std::mem::size_of::<GpuUniforms>().is_multiple_of(16));

impl GpuUniforms {
    pub fn from_view(
        view: &View,
        reference: &Reference,
        resolution: [u32; 2],
        palette: Palette,
    ) -> Self {
        Self {
            // Subtract in f64, then narrow: the difference is small even when
            // neither operand is, which is exactly what makes f32 viable here.
            ref_offset: (view.center - reference.center).to_f32_pair(),
            scale: view.scale as f32,
            max_iter: view.max_iter(),
            resolution: [resolution[0] as f32, resolution[1] as f32],
            palette: palette.index(),
            ref_len: reference.orbit.len() as u32,
        }
    }
}
