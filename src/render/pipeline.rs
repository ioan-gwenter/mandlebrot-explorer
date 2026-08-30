//! Construction of the fractal pass's GPU objects.
//!
//! Split out of the renderer so that `FractalRenderer` reads as "what happens
//! each frame" rather than opening with a wall of descriptor literals.

use crate::fractal::reference::MAX_REF;
use crate::render::context::Gpu;
use crate::render::uniforms::GpuUniforms;

/// One orbit sample as the shader sees it: `vec2<f32>`.
type OrbitPoint = [f32; 2];

/// Binding indices, matching the `@binding` attributes in the WGSL.
const BINDING_UNIFORMS: u32 = 0;
const BINDING_ORBIT: u32 = 1;

pub struct Resources {
    pub pipeline: wgpu::RenderPipeline,
    pub uniform_buffer: wgpu::Buffer,
    pub orbit_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
}

/// Build everything the fractal pass needs, in one place.
pub fn build(gpu: &Gpu) -> Resources {
    let shader = gpu
        .device
        .create_shader_module(wgpu::include_wgsl!("../shaders/mandelbrot.wgsl"));

    let uniform_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("mandelbrot uniforms"),
        size: size_of::<GpuUniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Sized for the longest orbit we will ever compute, so it never reallocates.
    let orbit_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("reference orbit"),
        size: (MAX_REF * size_of::<OrbitPoint>()) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group_layout = create_bind_group_layout(gpu);

    let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("mandelbrot bind group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: BINDING_UNIFORMS,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: BINDING_ORBIT,
                resource: orbit_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline = create_pipeline(gpu, &shader, &bind_group_layout);

    Resources {
        pipeline,
        uniform_buffer,
        orbit_buffer,
        bind_group,
    }
}

fn create_bind_group_layout(gpu: &Gpu) -> wgpu::BindGroupLayout {
    gpu.device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mandelbrot bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: BINDING_UNIFORMS,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        // Makes wgpu reject a Rust/WGSL layout mismatch at
                        // pipeline creation rather than rendering garbage.
                        min_binding_size: wgpu::BufferSize::new(size_of::<GpuUniforms>() as u64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: BINDING_ORBIT,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
}

fn create_pipeline(
    gpu: &Gpu,
    shader: &wgpu::ShaderModule,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let layout = gpu
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mandelbrot pipeline layout"),
            bind_group_layouts: &[Some(bind_group_layout)],
            immediate_size: 0,
        });

    gpu.device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mandelbrot pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                // No vertex buffers: the vertex shader generates a covering
                // triangle from the vertex index alone.
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        })
}
