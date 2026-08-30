# mandelbrot-explorer

GPU-driven Mandelbrot set explorer written in Rust, using `wgpu` for rendering
and `egui` for navigation menu. 

Deep level zooms stay precise via perturbation theory around a high-precision reference orbit, rather than
losing detail to `f64` rounding.

## Running

```sh
cargo run --release
```

## Controls

| Input                     | Effect                                   |
|----------------------------|-------------------------------------------|
| Left-click drag            | Pan the view                             |
| Scroll wheel / trackpad    | Zoom in/out, centered on the cursor      |
| `R`                        | Reset to the home view                   |
| `P`                        | Cycle through color palettes             |
| `C`                        | Log the current center coordinates       |
| `]` / `[`                  | Increase / decrease the iteration bias   |
| `Esc`                      | Quit                                     |

## Navigation panel

Navigation window  extends the keyboard/mouse controls:

- **re / im**: drag or type the current center coordinates.
- **detail** : slider controlling the iteration bias (same value as `]`/`[`).
- **palette**: dropdown to select specific colour palette
- **jump to**: type exact re/im values and click **go** to jump to a coordinate.
- **reset**  : return to the home view.
