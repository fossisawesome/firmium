// Visualizer Particle Shader (Scope particle field)
//
// Instanced glowing quads for the dust drifting out from the oscilloscope ring
// (the NCS / Wav2Bar look). One quad per particle; additive blending (set on the
// pipeline) makes overlapping particles accumulate into bright spots.
//
// Reads the particle storage buffer — two vec4 per particle, [0] = (x, y, size,
// alpha) and [1] = (colour_t, _, _, _), in normalized ring-space where radius
// 1.0 = half the panel — and maps it to pixels via the viewport. Only
// `viewport` + `gradient_colors` are read from the
// shared uniform, so a truncated Uniforms struct is declared (binding the full
// uniform buffer to a smaller struct is valid; the tail is simply not accessed).

struct Uniforms {
    viewport: vec4<f32>,                 // x, y, width, height in PIXELS
    gradient_colors: array<vec4<f32>, 8>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
// Two vec4 per particle: [0] = (x, y, size, alpha), [1] = (colour_t, _, _, _).
@group(0) @binding(4) var<storage, read> particles: array<vec4<f32>>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) quad: vec2<f32>, // -1..1 within the quad, for the radial falloff
    @location(1) color: vec4<f32>,
}

// Sample the 8-entry theme gradient at t in [0,1] (7 segments).
fn sample_gradient(t: f32) -> vec4<f32> {
    let tt = clamp(t, 0.0, 1.0) * 7.0;
    let i = u32(floor(tt));
    let f = tt - floor(tt);
    let i0 = min(i, 7u);
    let i1 = min(i + 1u, 7u);
    return mix(uniforms.gradient_colors[i0], uniforms.gradient_colors[i1], f);
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let p0 = particles[instance_index * 2u];
    let p1 = particles[instance_index * 2u + 1u];
    let pos = p0.xy;     // normalized ring-space (1.0 = half-panel per axis)
    let size = p0.z;     // normalized radius
    let alpha = p0.w;
    let color_t = p1.x;  // position along the gradient palette

    // Two-triangle quad corners for vertex_index 0..6.
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
    );
    let corner = corners[vertex_index];

    let vp = uniforms.viewport;
    let center = vec2<f32>(vp.z * 0.5, vp.w * 0.5);
    let half_min = min(vp.z, vp.w) * 0.5;
    // Position anisotropically (x by half-width, y by half-height) so the dust
    // field fills the whole panel: on a non-square (stretched) cover it reaches
    // the long-axis edges instead of sitting in a centered square, matching the
    // ring's stretch. Square panels: half-width == half-height == half_min, so
    // this is unchanged.
    let center_px = center + pos * vec2<f32>(vp.z * 0.5, vp.w * 0.5);
    // Size stays isotropic (half_min) so the dots stay round, not stretched ovals.
    let size_px = max(size * half_min, 0.5);
    let px = center_px + corner * size_px;

    // Pixel → NDC (flip Y for screen space).
    let ndc = vec2<f32>((px.x / vp.z) * 2.0 - 1.0, 1.0 - (px.y / vp.w) * 2.0);

    var col = sample_gradient(color_t);
    col.a = col.a * alpha;

    out.position = vec4<f32>(ndc.x, ndc.y, 0.0, 1.0);
    out.quad = corner;
    out.color = col;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Soft round falloff: bright core fading to nothing at the quad edge.
    let d = length(in.quad);
    let glow = clamp(1.0 - d, 0.0, 1.0);
    let intensity = glow * glow; // squared = softer, rounder dot
    var col = in.color;
    col.a = col.a * intensity;
    return col;
}
