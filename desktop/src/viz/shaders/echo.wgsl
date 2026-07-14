// Visualizer echo — Milkdrop-style zoom/rotate feedback.
//
// Each frame the persistent echo accumulator is copied to a scratch texture
// (so the warped read never aliases the write). This pass samples that scratch
// at a warped UV (rotate + zoom about the centre), fades it by `decay`, and
// maxes the current scene on top — producing spiralling / zooming feedback
// trails. The output is written back into the accumulator (Rgba16Float, so the
// fade has no 8-bit quantization floor). `decay = 0` clears stale content (used
// on the off->on transition).

struct EchoParams {
    decay: f32,
    zoom: f32,   // > 1 pulls the feedback toward the centre (tunnel-in)
    sin_a: f32,  // precomputed CPU-side: rotation per frame
    cos_a: f32,
}

@group(0) @binding(0) var scene_tex: texture_2d<f32>;
@group(0) @binding(1) var prev_tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;
@group(0) @binding(3) var<uniform> echo: EchoParams;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_echo(@builtin(vertex_index) idx: u32) -> VsOut {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var out: VsOut;
    out.position = vec4<f32>(pos[idx], 0.0, 1.0);
    out.uv = pos[idx] * vec2<f32>(0.5, -0.5) + 0.5;
    return out;
}

@fragment
fn fs_echo(in: VsOut) -> @location(0) vec4<f32> {
    let c = vec2<f32>(0.5, 0.5);
    var p = in.uv - c;
    // Rotate about the centre.
    p = vec2<f32>(
        p.x * echo.cos_a - p.y * echo.sin_a,
        p.x * echo.sin_a + p.y * echo.cos_a,
    );
    // Zoom about the centre.
    p = p * echo.zoom;
    let warped = c + p;

    let prev = textureSampleLevel(prev_tex, samp, warped, 0.0) * echo.decay;
    let scene = textureSampleLevel(scene_tex, samp, in.uv, 0.0);
    return max(scene, prev);
}
