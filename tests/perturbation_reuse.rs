//! End-to-end checks on reference-orbit reuse.
//!
//! The reference orbit is the expensive part of perturbation rendering: up to
//! 50,000 f64 iterations on the CPU, then up to 800 KB uploaded to the GPU.
//! The old staleness check compared view centres for exact float equality, so
//! *every* pan frame paid both costs. These tests pin down the reuse behaviour
//! that replaced it.

use mandelbrot_viewer::action::Action;
use mandelbrot_viewer::math::Complex;
use mandelbrot_viewer::scene::Scene;

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 800;

fn zoomed_scene(zoom_steps: usize) -> Scene {
    let mut scene = Scene::new(WIDTH, HEIGHT);
    // Zoom towards a point on the boundary, where orbits are long.
    let _ = scene.apply(Action::SetCenter(Complex::new(
        -0.743_643_887_037_151,
        0.131_825_904_205_33,
    )));
    for _ in 0..zoom_steps {
        let _ = scene.apply(Action::ZoomAt {
            px: WIDTH as f64 / 2.0,
            py: HEIGHT as f64 / 2.0,
            factor: 0.85,
        });
    }
    scene.sync_reference();
    scene
}

/// A drag is delivered as many small `CursorMoved` events. Under the old
/// exact-equality check every one of them regenerated the orbit.
#[test]
fn a_drag_regenerates_the_orbit_far_less_than_once_per_frame() {
    let mut scene = zoomed_scene(60);
    let start = scene.reference_generation();

    const FRAMES: usize = 600;
    for _ in 0..FRAMES {
        let _ = scene.apply(Action::Pan { dx: 2.0, dy: 1.0 });
        scene.sync_reference();
    }

    let regenerations = scene.reference_generation() - start;
    assert!(
        regenerations < FRAMES as u64 / 10,
        "a {FRAMES}-frame drag regenerated the orbit {regenerations} times; \
         the old exact-equality check would have regenerated it every frame"
    );
}

/// Reuse must not be unbounded: drag far enough and the reference has to move,
/// or the f32 deltas the shader works in lose precision.
#[test]
fn dragging_far_enough_does_eventually_regenerate() {
    let mut scene = zoomed_scene(60);
    let start = scene.reference_generation();

    // Several screen widths of travel.
    for _ in 0..5_000 {
        let _ = scene.apply(Action::Pan { dx: 5.0, dy: 0.0 });
        scene.sync_reference();
    }

    assert!(
        scene.reference_generation() > start,
        "the reference must follow the view when it travels a long way"
    );
}

/// Zooming without moving keeps the reference centred, so it stays usable
/// until the iteration budget outgrows the stored orbit.
#[test]
fn zooming_in_place_reuses_the_orbit_until_max_iter_outgrows_it() {
    let mut scene = Scene::new(WIDTH, HEIGHT);
    scene.sync_reference();
    let start = scene.reference_generation();

    for _ in 0..30 {
        let _ = scene.apply(Action::ZoomAt {
            px: WIDTH as f64 / 2.0,
            py: HEIGHT as f64 / 2.0,
            factor: 0.85,
        });
        scene.sync_reference();
    }

    let regenerations = scene.reference_generation() - start;
    assert!(
        regenerations <= 30,
        "expected at most one regeneration per zoom step, got {regenerations}"
    );
}

/// Idle frames must cost nothing: a redraw with no input regenerates nothing.
#[test]
fn idle_frames_never_regenerate() {
    let mut scene = zoomed_scene(40);
    let start = scene.reference_generation();

    for _ in 0..240 {
        scene.sync_reference();
    }

    assert_eq!(
        scene.reference_generation(),
        start,
        "idle redraws must not touch the reference orbit"
    );
}

/// The orbit is only re-uploaded when its generation changes, so the counter
/// must be monotonic — never reused for different contents.
#[test]
fn the_generation_counter_only_moves_forward() {
    let mut scene = Scene::new(WIDTH, HEIGHT);
    let mut last = scene.reference_generation();

    for step in 0..200 {
        match step % 3 {
            0 => {
                let _ = scene.apply(Action::Pan { dx: 3.0, dy: -2.0 });
            }
            1 => {
                let _ = scene.apply(Action::ZoomAt {
                    px: 100.0,
                    py: 100.0,
                    factor: 0.9,
                });
            }
            _ => {
                let _ = scene.apply(Action::ScaleIterBias(1.5));
            }
        }
        scene.sync_reference();

        let now = scene.reference_generation();
        assert!(now >= last, "generation went backwards: {last} -> {now}");
        last = now;
    }
}
