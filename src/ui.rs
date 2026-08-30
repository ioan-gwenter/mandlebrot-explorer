//! The explorer panel's layout.

use crate::action::Action;
use crate::fractal::palette::Palette;
use crate::fractal::view::View;
use crate::math::Complex;
use crate::scene::Scene;

/// Scratch state belonging to the panel itself
#[derive(Default)]
pub struct PanelState {
    re_text: String,
    im_text: String,
    /// Why the last "go" was refused, shown under the fields.
    jump_error: Option<&'static str>,
}

/// Build the panel for this frame, collecting whatever the user asked for.
pub fn explorer_panel(ctx: &egui::Context, scene: &Scene, panel: &mut PanelState) -> Vec<Action> {
    let mut actions = Vec::new();
    let view = &scene.view;

    egui::Window::new("Navigation")
        .default_pos([12.0, 12.0])
        .resizable(false)
        .show(ctx, |ui| {
            // Drag sensitivity tracks the zoom, so a drag moves a similar
            // on-screen distance at any depth.
            let speed = view.scale * 2.0;
            let (mut re, mut im) = (view.center.re, view.center.im);
            let mut moved = false;

            ui.horizontal(|ui| {
                ui.label("re");
                moved |= ui
                    .add(egui::DragValue::new(&mut re).speed(speed).max_decimals(17))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("im");
                moved |= ui
                    .add(egui::DragValue::new(&mut im).speed(speed).max_decimals(17))
                    .changed();
            });
            if moved {
                actions.push(Action::SetCenter(Complex::new(re, im)));
            }

            ui.label(format!("zoom  {:.4e}x", view.zoom_level()));
            ui.label(format!("iter  {}", view.max_iter()));

            let mut bias = view.iter_bias;
            if ui
                .add(
                    egui::Slider::new(&mut bias, View::ITER_BIAS_RANGE)
                        .logarithmic(true)
                        .text("detail"),
                )
                .changed()
            {
                actions.push(Action::SetIterBias(bias));
            }

            let mut palette = scene.palette;
            egui::ComboBox::from_label("palette")
                .selected_text(palette.name())
                .show_ui(ui, |ui| {
                    for option in Palette::ALL {
                        if ui
                            .selectable_value(&mut palette, option, option.name())
                            .changed()
                        {
                            actions.push(Action::SetPalette(option));
                        }
                    }
                });

            ui.collapsing("jump to", |ui| {
                ui.horizontal(|ui| {
                    ui.label("re");
                    ui.text_edit_singleline(&mut panel.re_text);
                });
                ui.horizontal(|ui| {
                    ui.label("im");
                    ui.text_edit_singleline(&mut panel.im_text);
                });
                if ui.button("go").clicked() {
                    match parse_center(&panel.re_text, &panel.im_text) {
                        Ok(target) => {
                            panel.jump_error = None;
                            actions.push(Action::SetCenter(target));
                        }
                        Err(why) => panel.jump_error = Some(why),
                    }
                }
                if let Some(why) = panel.jump_error {
                    ui.colored_label(ui.visuals().error_fg_color, why);
                }
            });

            if ui.button("reset").clicked() {
                actions.push(Action::ResetView);
            }
        });

    actions
}

fn parse_center(re: &str, im: &str) -> Result<Complex, &'static str> {
    Ok(Complex::new(
        parse_coord(re).ok_or("re is not a finite number")?,
        parse_coord(im).ok_or("im is not a finite number")?,
    ))
}

fn parse_coord(text: &str) -> Option<f64> {
    text.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_well_formed_pair() {
        assert_eq!(parse_center(" -0.75 ", "0.1"), Ok(Complex::new(-0.75, 0.1)));
    }

    #[test]
    fn rejects_a_partial_or_malformed_pair() {
        for (re, im) in [
            ("-0.75", ""),
            ("", "0.1"),
            ("abc", "0.1"),
            ("-0.75", "1.2.3"),
        ] {
            assert!(parse_center(re, im).is_err(), "{re:?}, {im:?}");
        }
    }

    /// `"nan"`, `"inf"` and an overflowing exponent all parse happily as `f64`.
    /// A NaN centre would rebuild the reference orbit every frame forever, so
    /// these have to be turned away.
    #[test]
    fn rejects_input_that_parses_but_is_not_finite() {
        for (re, im) in [
            ("nan", "0.1"),
            ("0.1", "NaN"),
            ("inf", "0.1"),
            ("0.1", "-Infinity"),
            ("1e400", "0.1"),
            ("0.1", "-1e400"),
        ] {
            assert!(parse_center(re, im).is_err(), "{re:?}, {im:?}");
        }
    }

    #[test]
    fn accepts_scientific_notation() {
        assert_eq!(
            parse_center("-7.436e-1", "1.318e-1"),
            Ok(Complex::new(-7.436e-1, 1.318e-1))
        );
    }
}
