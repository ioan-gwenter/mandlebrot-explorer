// Mirrors `GpuUniforms` in src/uniforms.rs — keep both in lockstep.
struct Uniforms {
    ref_offset: vec2<f32>,  // view_center - ref_center
    scale: f32,
    max_iter: u32,
    resolution: vec2<f32>,
    palette: u32,
    ref_len: u32,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var<storage, read> ref_orbit: array<vec2<f32>>;

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(i) / 2) * 4.0 - 1.0;
    let y = f32(i32(i) & 1) * 4.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

const TAU: f32 = 6.283185307;

fn palette(t: f32, mode: u32) -> vec3<f32> {
    var a = vec3<f32>(0.5, 0.5, 0.5);
    var b = vec3<f32>(0.5, 0.5, 0.5);
    var c = vec3<f32>(1.0, 1.0, 1.0);
    var d = vec3<f32>(0.00, 0.33, 0.67); // rainbow

    switch mode {
        case 1u: { d = vec3<f32>(0.00, 0.10, 0.20); }              // ember
        case 2u: {
            c = vec3<f32>(1.0, 1.0, 0.5);
            d = vec3<f32>(0.80, 0.90, 0.30);                        // ocean
        }
        case 3u: {                                                 // grey
            b = vec3<f32>(0.5, 0.5, 0.5);
            c = vec3<f32>(1.0, 1.0, 1.0);
            d = vec3<f32>(0.0, 0.0, 0.0);
        }
        default: {}
    }

    return a + b * cos(TAU * (c * t + d));
}

fn cmul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<f32>(frag.x - u.resolution.x * 0.5,
        u.resolution.y * 0.5 - frag.y);

    let dc = u.ref_offset + p * u.scale;

    var dz = vec2<f32>(0.0, 0.0);
    var z = vec2<f32>(0.0, 0.0);
    var m: u32 = 0u; // index into the reference orbit
    var n: u32 = 0u; // true iteration count

    loop {
        if n >= u.max_iter { break; }

        z = ref_orbit[m] + dz;
        if dot(z, z) > 65536.0 { break; }

        if dot(z, z) < dot(dz, dz) || m + 1u >= u.ref_len {
            dz = z;
            m = 0u;
        }

        // delta_{n+1} = 2*Z_m*delta + delta^2 + dc
        let zm = ref_orbit[m];
        dz = 2.0 * cmul(zm, dz) + cmul(dz, dz) + dc;

        m = m + 1u;
        n = n + 1u;
    }

    if n >= u.max_iter {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let sn = f32(n) - log2(log2(dot(z, z))) + 4.0;
    let t = sqrt(max(sn, 0.0)) * 0.14;
    return vec4<f32>(palette(t, u.palette), 1.0);
}