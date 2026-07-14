// Visualizer CRT / film composite.
//
// A single post-process pass over the displayed scene (resolve / trail / echo —
// bound at @group(0), matching the blit bind-group layout). Applies a retro
// stack scaled by one master `amount`: radial chromatic aberration, scanlines,
// vignette, film grain, plus a beat zoom-punch. No geometric screen curvature —
// it is kept 1:1 so the visualizer stays pixel-sharp and snug to the window edge
// (see fs_crt). Writes the framebuffer with the same premultiplied-alpha blend as
// the plain blit.
//
// Everything is gated to the visualizer's own content (multiplicative effects
// stay 0 on the premultiplied-transparent background; grain is × alpha) so the
// effect never tints the UI behind the overlay.

struct CrtParams {
    amount: f32, // master retro intensity (0 = off)
    beat: f32,   // beat pulse for the zoom-punch
    time: f32,   // seconds, for grain + scanline scroll
    _pad: f32,
}

@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(1) @binding(0) var<uniform> crt: CrtParams;

const PI: f32 = 3.14159265;
const CRT_CA: f32 = 0.004;
const CRT_SCANLINE: f32 = 0.25;
const CRT_VIGNETTE: f32 = 0.9;
const CRT_GRAIN: f32 = 0.07;
const CRT_BEAT_ZOOM: f32 = 0.02;
const CRT_SCAN_SCROLL: f32 = 12.0;
// Fixed scanline count so density is resolution-independent and stays well
// under the framebuffer Nyquist limit (tying it to physical texture height
// aliases/shimmers on HiDPI / fractional-scaled displays).
const CRT_SCANLINE_COUNT: f32 = 320.0;

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_crt(@builtin(vertex_index) idx: u32) -> VsOut {
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
fn fs_crt(in: VsOut) -> @location(0) vec4<f32> {
    let amount = crt.amount;
    let c = vec2<f32>(0.5, 0.5);

    // Beat zoom-punch: pull the image in slightly on the kick.
    let uv = c + (in.uv - c) * (1.0 - crt.beat * CRT_BEAT_ZOOM * amount);

    // No geometric barrel curve. Warping the finished image through the bilinear
    // blit sampler is inherently soft — magnified taps blend texels — and the only
    // artifact-free fit (over-scanning to refit the warp inside the frame) zooms
    // the image ~5% and reads as blur, while clamping the over-scan band smears a
    // flat edge line and masking it opens a transparent gap. Since sharp + snug +
    // artifact-free is mutually exclusive with any post-process pixel warp, the
    // frame is kept 1:1 for pixel-sharp, edge-snug output and the surviving retro
    // stack (listed in the header) carries the look.
    let d = uv - c; // radius vector from center; feeds the vignette and the CA split
    let r2 = dot(d, d); // squared radius

    // Radial chromatic aberration: split R/G/B along the radius `d`.
    let ca = CRT_CA * amount;
    let sr = textureSampleLevel(tex, samp, uv + d * ca, 0.0);
    let sg = textureSampleLevel(tex, samp, uv, 0.0);
    let sb = textureSampleLevel(tex, samp, uv - d * ca, 0.0);
    var col = vec3<f32>(sr.r, sg.g, sb.b);
    let alpha = sg.a;

    let dims = vec2<f32>(textureDimensions(tex));

    // Scanlines.
    let scan_count = min(CRT_SCANLINE_COUNT, dims.y * 0.5);
    let scan = 0.5 + 0.5 * sin(uv.y * scan_count * PI + crt.time * CRT_SCAN_SCROLL);
    col = col * mix(1.0, scan, CRT_SCANLINE * amount);

    // Vignette (darken toward the corners).
    let vig = 1.0 - clamp(CRT_VIGNETTE * amount * r2 * 2.0, 0.0, 1.0);
    col = col * vig;

    // Film grain — × alpha so it only lands on the visualizer's content.
    let grain = (hash(uv * dims + crt.time) - 0.5) * CRT_GRAIN * amount;
    col = col + vec3<f32>(grain * alpha);

    return vec4<f32>(clamp(col, vec3<f32>(0.0), vec3<f32>(1.0)), alpha);
}
