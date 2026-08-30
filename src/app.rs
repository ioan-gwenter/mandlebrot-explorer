//! winit application wiring.

use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowId};

use crate::action::Action;
use crate::input::InputState;
use crate::render::context::Gpu;
use crate::render::hud::Hud;
use crate::render::mandelbrot::FractalRenderer;
use crate::scene::Scene;

const WINDOW_TITLE: &str = "Mandelbrot Set Explorer";
const INITIAL_SIZE: (u32, u32) = (1280, 800);

#[derive(Default)]
pub struct App {
    state: Option<State>,
}

struct State {
    gpu: Gpu,
    renderer: FractalRenderer,
    hud: Hud,
    scene: Scene,
    input: InputState,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_inner_size(winit::dpi::LogicalSize::new(INITIAL_SIZE.0, INITIAL_SIZE.1));

        let window = match event_loop.create_window(attrs) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                log::error!("could not create window: {err}");
                event_loop.exit();
                return;
            }
        };

        let gpu = pollster::block_on(Gpu::new(window.clone()));
        let scene = Scene::new(gpu.config.width, gpu.config.height);
        let renderer = FractalRenderer::new(&gpu);
        let hud = Hud::new(&gpu, window.clone());

        window.request_redraw();

        self.state = Some(State {
            gpu,
            renderer,
            hud,
            scene,
            input: InputState::default(),
        });
    }

    /// Wake the window when a deadline set for egui comes due 
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if let StartCause::ResumeTimeReached { .. } = cause
            && let Some(state) = self.state.as_ref()
        {
            state.gpu.window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        // egui gets first refusal on every event.
        let response = state.hud.on_window_event(&state.gpu.window, &event);

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::Resized(size) => {
                state.gpu.resize(size.width, size.height);
                state.scene.resize(size.width, size.height);
                state.gpu.window.request_redraw();
                return;
            }
            WindowEvent::RedrawRequested => {
                state.render(event_loop);
                return;
            }
            _ => {}
        }

        if response.repaint {
            state.gpu.window.request_redraw();
        }

        if response.consumed || state.hud.wants_pointer() {
            state.input.cancel_drag();
            return;
        }

        if let Some(action) = state.input.handle(&event) {
            state.dispatch(action, event_loop);
        }
    }
}

impl State {
    /// Apply one action, redrawing if it changed anything visible.
    fn dispatch(&mut self, action: Action, event_loop: &ActiveEventLoop) {
        if action.is_app_level() {
            debug_assert_eq!(action, Action::Exit);
            event_loop.exit();
            return;
        }

        if self.scene.apply(action).is_needed() {
            self.gpu.window.request_redraw();
        }
    }

    fn render(&mut self, event_loop: &ActiveEventLoop) {
        let Some(frame) = self.gpu.acquire() else {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        };
        let target = frame.texture.create_view(&Default::default());
        let mut encoder = self.gpu.device.create_command_encoder(&Default::default());

        self.scene.sync_reference();
        self.renderer
            .encode(&self.gpu, &mut encoder, &target, &self.scene);

        let hud = self
            .hud
            .render(&self.gpu, &mut encoder, &target, &self.scene);

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        self.gpu.queue.present(frame);

        // Applied after presenting so the panel's own frame is already on
        // screen; anything the user asked for lands on the next one.
        let mut scene_changed = false;
        for action in hud.actions {
            if action.is_app_level() {
                self.dispatch(action, event_loop);
                return;
            }
            scene_changed |= self.scene.apply(action).is_needed();
        }

        self.schedule_next_frame(event_loop, scene_changed, hud.repaint_after);
    }

    fn schedule_next_frame(
        &self,
        event_loop: &ActiveEventLoop,
        scene_changed: bool,
        repaint_after: Duration,
    ) {
        if scene_changed || repaint_after.is_zero() {
            self.gpu.window.request_redraw();
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }

        match Instant::now().checked_add(repaint_after) {
            Some(deadline) => event_loop.set_control_flow(ControlFlow::WaitUntil(deadline)),
            None => event_loop.set_control_flow(ControlFlow::Wait),
        }
    }
}
