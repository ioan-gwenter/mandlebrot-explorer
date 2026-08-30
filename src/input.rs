//! Translates winit events into [`Action`]s.

use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use crate::action::Action;

/// Zoom applied per wheel notch.
const ZOOM_PER_NOTCH: f64 = 0.85;

/// Trackpad pixels equivalent to one wheel notch.
const PIXELS_PER_NOTCH: f64 = 60.0;

/// Multiplier applied to `iter_bias`
const ITER_BIAS_STEP: f64 = 1.5;

#[derive(Default)]
pub struct InputState {
    cursor: (f64, f64),
    dragging: bool,
}

impl InputState {
   
    pub fn cancel_drag(&mut self) {
        self.dragging = false;
    }

   
    pub fn handle(&mut self, event: &WindowEvent) -> Option<Action> {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let action = self.dragging.then_some(Action::Pan {
                    dx: position.x - self.cursor.0,
                    dy: position.y - self.cursor.1,
                });
                self.cursor = (position.x, position.y);
                action
            }

            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                self.dragging = *state == ElementState::Pressed;
                None
            }

            WindowEvent::CursorLeft { .. } => {
                self.cancel_drag();
                None
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let notches = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y as f64,
                    MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => {
                        y / PIXELS_PER_NOTCH
                    }
                };
                Some(Action::ZoomAt {
                    px: self.cursor.0,
                    py: self.cursor.1,
                    factor: ZOOM_PER_NOTCH.powf(notches),
                })
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => Self::key_action(*code),

            _ => None,
        }
    }

    fn key_action(code: KeyCode) -> Option<Action> {
        Some(match code {
            KeyCode::Escape => Action::Exit,
            KeyCode::KeyR => Action::ResetView,
            KeyCode::KeyP => Action::CyclePalette,
            KeyCode::KeyC => Action::LogPosition,
            KeyCode::BracketRight => Action::ScaleIterBias(ITER_BIAS_STEP),
            KeyCode::BracketLeft => Action::ScaleIterBias(1.0 / ITER_BIAS_STEP),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bracket_keys_are_exact_inverses() {
        let (Some(Action::ScaleIterBias(up)), Some(Action::ScaleIterBias(down))) = (
            InputState::key_action(KeyCode::BracketRight),
            InputState::key_action(KeyCode::BracketLeft),
        ) else {
            panic!("bracket keys should scale the iteration bias");
        };
        assert!((up * down - 1.0).abs() < 1e-12);
    }

    #[test]
    fn unmapped_keys_ask_for_nothing() {
        assert_eq!(InputState::key_action(KeyCode::KeyZ), None);
    }

    #[test]
    fn scrolling_up_zooms_in_and_down_zooms_out() {
        let zoom_in = ZOOM_PER_NOTCH.powf(1.0);
        let zoom_out = ZOOM_PER_NOTCH.powf(-1.0);
        assert!(zoom_in < 1.0, "scrolling up should shrink scale");
        assert!(zoom_out > 1.0, "scrolling down should grow scale");
    }
}
