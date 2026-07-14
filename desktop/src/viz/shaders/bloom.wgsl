// Visualizer bloom post-processing.
//
// The scene (bars / lines) is first rendered into the widget-sized resolve
// texture (see `VisualizerPrimitive::render` in shader.rs). These passes then:
//   1. fs_bright_h — downsample the resolve texture into the half-res bloom
//      target with a horizontal Gaussian blur + a soft-knee brightness
//      threshold (so only the bright parts — bar tops, peak flashes, the neon
//      line core — bloom).
//   2. fs_blur_v   — vertical Gaussian blur of the half-res bloom target.
//   3. fs_composite — additively composite the blurred glow back over the
//      scene, scaled by the user's bloom intensity.
//
// There is no HDR framebuffer, so everything stays in the surface format. The
// resolve texture holds premultiplied-alpha color (it is rendered onto a
// transparent-cleared target), which is the correct space to blur and to add
// additively, so no un/re-premultiply dance is needed here.
//
// v1 limitation: bloom samples the live resolve texture (current frame's bars /
// lines), not the trail / echo accumulator. So when bloom is stacked with
// trails or echo, the halo tracks the current frame, not the fading after-image
// — faded comet trails / echo tunnels get no glow. Subtle by construction (the
// brightest, most bloom-prone pixels are the live scene; the un-haloed history
// is the dim part least likely to bloom), so it ships as-is. A true fix would
// move the bright/blur passes to run after the trail/echo accumulation and add
// a bloom bind group sampling whichever accumulator is displayed.

struct BloomParams {
    intensity: f32, // additive strength of the glow (composite pass)
    threshold: f32, // soft-knee brightness cutoff (downsample pass)
    _pad0: f32,
    _pad1: f32,
}

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;
@group(0) @binding(2) var<uniform> params: BloomParams;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VsOut {
    // Full-viewport triangle.
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var out: VsOut;
    out.position = vec4<f32>(pos[idx], 0.0, 1.0);
    // NDC -> UV (flip Y for texture sampling).
    out.uv = pos[idx] * vec2<f32>(0.5, -0.5) + 0.5;
    return out;
}

// 9-tap Gaussian weights (normalized; sum = 1).
const W0: f32 = 0.227027;
const W1: f32 = 0.1945946;
const W2: f32 = 0.1216216;
const W3: f32 = 0.054054;
const W4: f32 = 0.016216;

// Separable Gaussian stepping `dir` source-texels per tap. `dir` is NOT
// necessarily unit length: pass 1 reads the full-res resolve while pass 2 reads
// the half-res bloom target, so pass 1 doubles its step to keep the blur radius
// equal on both axes (otherwise the glow halo is 2x taller than it is wide).
fn blur_axis(uv: vec2<f32>, dir: vec2<f32>) -> vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(src_tex));
    let off = dir * texel;
    var c = textureSampleLevel(src_tex, src_sampler, uv, 0.0) * W0;
    c += textureSampleLevel(src_tex, src_sampler, uv + off * 1.0, 0.0) * W1;
    c += textureSampleLevel(src_tex, src_sampler, uv - off * 1.0, 0.0) * W1;
    c += textureSampleLevel(src_tex, src_sampler, uv + off * 2.0, 0.0) * W2;
    c += textureSampleLevel(src_tex, src_sampler, uv - off * 2.0, 0.0) * W2;
    c += textureSampleLevel(src_tex, src_sampler, uv + off * 3.0, 0.0) * W3;
    c += textureSampleLevel(src_tex, src_sampler, uv - off * 3.0, 0.0) * W3;
    c += textureSampleLevel(src_tex, src_sampler, uv + off * 4.0, 0.0) * W4;
    c += textureSampleLevel(src_tex, src_sampler, uv - off * 4.0, 0.0) * W4;
    return c;
}

// Pass 1: horizontal blur + soft-knee threshold (resolve -> half-res bloom).
// The source is the FULL-res resolve, so step 2 source-texels per tap to match
// the half-res step pass 2 takes — keeps the halo circular, not stretched.
@fragment
fn fs_bright_h(in: VsOut) -> @location(0) vec4<f32> {
    let blurred = blur_axis(in.uv, vec2<f32>(2.0, 0.0));
    let luma = dot(blurred.rgb, vec3<f32>(0.299, 0.587, 0.114));
    let knee = smoothstep(params.threshold, params.threshold + 0.25, luma);
    return blurred * knee;
}

// Pass 2: vertical blur (half-res bloom -> half-res bloom).
@fragment
fn fs_blur_v(in: VsOut) -> @location(0) vec4<f32> {
    return blur_axis(in.uv, vec2<f32>(0.0, 1.0));
}

// Horizontal blur WITHOUT the threshold (half-res bloom -> half-res bloom). Used
// by the wide-glow iterations: after the bright/threshold pass has extracted the
// glow, each extra (H, V) iteration just re-blurs the half-res buffer to widen
// the halo while staying smooth. Steps 1 half-res texel/tap to match fs_blur_v.
@fragment
fn fs_blur_h(in: VsOut) -> @location(0) vec4<f32> {
    return blur_axis(in.uv, vec2<f32>(1.0, 0.0));
}

// Pass 3: additive composite of the glow over the scene, scaled by intensity.
// Paired with a One/One blend state on the framebuffer target.
@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    let glow = textureSampleLevel(src_tex, src_sampler, in.uv, 0.0);
    return glow * params.intensity;
}
