//! egui overlay plumbing: feed winit events in, tessellate, draw over the
//! fractal pass.

use std::sync::Arc;
use std::time::Duration;
use winit::window::Window;

use crate::action::Action;
use crate::render::context::Gpu;
use crate::scene::Scene;
use crate::ui::{self, PanelState};

pub struct Hud {
    ctx: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    panel: PanelState,
}

pub struct HudOutput {
    /// What the user asked for this frame.
    pub actions: Vec<Action>,
    /// How long egui can go before it needs another frame: zero while something
    /// is animating, [`Duration::MAX`] when the panel is idle and waiting on the
    /// user, and a finite delay for timed work such as the blinking text caret.
    pub repaint_after: Duration,
}

impl Hud {
    pub fn new(gpu: &Gpu, window: Arc<Window>) -> Self {
        let ctx = egui::Context::default();
        let state = egui_winit::State::new(
            ctx.clone(),
            egui::ViewportId::ROOT,
            &*window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            gpu.config.format,
            egui_wgpu::RendererOptions {
                msaa_samples: 1,
                depth_stencil_format: None,
                dithering: false,
                ..Default::default()
            },
        );
        Self {
            ctx,
            state,
            renderer,
            panel: PanelState::default(),
        }
    }

    /// Offer an event to egui.
    pub fn on_window_event(
        &mut self,
        window: &Window,
        event: &winit::event::WindowEvent,
    ) -> egui_winit::EventResponse {
        self.state.on_window_event(window, event)
    }

    /// Whether the pointer is over egui, so canvas gestures must be suppressed.
    pub fn wants_pointer(&self) -> bool {
        self.ctx.egui_wants_pointer_input()
    }

    /// Build the panel and encode its draw commands over `target`.
    pub fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        scene: &Scene,
    ) -> HudOutput {
        let raw_input = self.state.take_egui_input(&gpu.window);

        let mut actions = Vec::new();
        let ctx = self.ctx.clone();
        let mut full_output = ctx.run_ui(raw_input, |ctx| {
            actions = ui::explorer_panel(ctx, scene, &mut self.panel);
        });

        self.state
            .handle_platform_output(&gpu.window, full_output.platform_output);

        let repaint_after = full_output
            .viewport_output
            .values()
            .map(|v| v.repaint_delay)
            .min()
            .unwrap_or(Duration::MAX);

        let pixels_per_point = full_output.pixels_per_point;
        let triangles = self.ctx.tessellate(full_output.shapes, pixels_per_point);

        for (id, deltas) in &full_output.textures_delta.set {
            for delta in deltas {
                self.renderer
                    .update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }

        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [gpu.config.width, gpu.config.height],
            pixels_per_point,
        };
        self.renderer
            .update_buffers(&gpu.device, &gpu.queue, encoder, &triangles, &screen);

        {
            // Load rather than clear: the fractal pass has already drawn here.
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.renderer
                .render(&mut pass.forget_lifetime(), &triangles, &screen);
        }

        for id in &full_output.textures_delta.free {
            self.renderer.free_texture(id);
        }
        full_output.textures_delta.clear();

        HudOutput {
            actions,
            repaint_after,
        }
    }
}
