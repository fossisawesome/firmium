// Visualizer Scope Shader (circular oscilloscope)
// Maps the time-domain waveform around a ring centered in the widget bounds.
// Reuses the Lines mode's color/glow/AA machinery; only the geometry differs
// (polar instead of linear, closed loop instead of an open polyline).
//
// `bar_data` here holds the SIGNED, normalized waveform (~-1..1), NOT the FFT
// magnitudes — the renderer uploads `get_waveform()` for this mode.
//
// ⚠️  Config struct layout MUST match VisualizerConfig in shader.rs byte-for-byte.
//     If you add/remove/reorder fields, update ALL locations:
//       1. src/widgets/visualizer/shader.rs          (VisualizerConfig)
//       2. src/widgets/visualizer/shaders/bars.wgsl  (Config)
//       3. src/widgets/visualizer/shaders/lines.wgsl (Config)
//       4. src/widgets/visualizer/shaders/scope.wgsl (Config)

struct Uniforms {
    viewport: vec4<f32>,  // x, y, width, height in PIXELS
    gradient_colors: array<vec4<f32>, 8>,
    peak_gradient_colors: array<vec4<f32>, 8>,
    peak_color: vec4<f32>,
    border_color: vec4<f32>,
    config: Config,
    audio: vec4<f32>,  // [beat * reactivity, bass, mid, treble]
}

struct Config {
    bar_count: u32,
    mode: u32,  // 0 = bars, 1 = lines, 2 = scope
    border_width: f32,
    peak_enabled: u32,
    peak_thickness: f32,
    peak_alpha: f32,
    line_thickness: f32,
    bar_width: f32,
    bar_spacing: f32,
    edge_spacing: f32,
    time: f32,
    led_bars: u32,
    led_segment_height: f32,
    led_border_opacity: f32,
    border_opacity: f32,
    gradient_mode: u32,
    peak_gradient_mode: u32,
    peak_mode: u32,
    peak_hold_time: f32,
    peak_fade_time: f32,
    flash_count: u32,
    bar_depth_3d: f32,
    gradient_orientation: u32,
    average_energy: f32,
    global_opacity: f32,
    lines_outline_thickness: f32,
    lines_outline_opacity: f32,
    lines_animation_speed: f32,
    lines_gradient_mode: u32,
    lines_fill_opacity: f32,
    lines_mirror: u32,
    lines_glow_intensity: f32,
    lines_style: u32,
    bars_flash_intensity: f32,
    scope_radius: f32,             // Scope mode: mean ring radius fraction
    scope_sensitivity: f32,        // Scope mode: waveform swing / gain
    flash_data: array<vec4<f32>, 512>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> bar_data: array<f32>;  // Signed waveform (-1..1)
@group(0) @binding(2) var<storage, read> peak_data: array<f32>;  // Unused in scope mode
@group(0) @binding(3) var<storage, read> peak_alpha_data: array<f32>;  // Unused in scope mode

const TAU: f32 = 6.28318530717958647692;

// Smooth spline samples per segment (one segment per waveform point). MUST match
// SCOPE_SAMPLES_PER_SEGMENT in shader.rs (the CPU draw-call vertex count).
const SCOPE_SP: i32 = 12;

// Ring sizing is user-controlled via the uniform:
//   config.scope_radius      mean ring radius as a fraction of the available
//                            space (after reserving stroke + glow margin);
//   config.scope_sensitivity waveform gain (how hard loud audio swings the ring).
// The deviation uses the remaining headroom `(1 - radius)` so a full-scale swing
// reaches the margin but never clips past the panel.
const SCOPE_RADIUS_MIN: f32 = 0.1;
const SCOPE_RADIUS_MAX: f32 = 0.95;

// ---------- Palette segment-count constants (mirrors lines.wgsl) ----------
const LINES_PALETTE_SEGMENTS_STATIC: f32 = 7.0;
const LINES_PALETTE_SEGMENTS_LOOPED: f32 = 8.0;
const LINES_PALETTE_INDEX_TAIL: u32 = 7u;
const LINES_PALETTE_INDEX_MOD: u32 = 8u;

// ---------- Neon glow constants/helpers (mirrors lines.wgsl) ----------
const LINES_GLOW_MIN_RADIUS: f32 = 3.0;
const LINES_GLOW_MAX_RADIUS: f32 = 10.0;
const LINES_GLOW_EXTENT_MULT: f32 = 2.5;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) frag_pos: vec2<f32>,  // Canvas-space pixel position (interpolated → SDF input)
    @location(2) is_outline: f32,  // 1.0 = outline, 0.0 = main line
    @location(3) is_fill: f32,  // 1.0 = fill pass, 0.0 = line/outline pass
    @location(4) @interpolate(flat) seg_a: vec2<f32>,  // This quad's ring segment start (canvas px)
    @location(5) @interpolate(flat) seg_b: vec2<f32>,  // This quad's ring segment end (canvas px)
}

fn dead_output() -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(-2.0, -2.0, 0.0, 1.0);
    output.color = vec4<f32>(0.0);
    output.frag_pos = vec2<f32>(0.0);
    output.is_outline = 0.0;
    output.is_fill = 0.0;
    output.seg_a = vec2<f32>(0.0);
    output.seg_b = vec2<f32>(0.0);
    return output;
}

fn pixel_to_ndc(pixel_x: f32, pixel_y: f32) -> vec2<f32> {
    let viewport = uniforms.viewport;
    let ndc_x = (pixel_x / viewport.z) * 2.0 - 1.0;
    let ndc_y = 1.0 - (pixel_y / viewport.w) * 2.0;
    return vec2<f32>(ndc_x, ndc_y);
}

fn lines_glow_radius() -> f32 {
    let s = clamp(uniforms.config.lines_glow_intensity, 0.0, 1.0);
    return mix(LINES_GLOW_MIN_RADIUS, LINES_GLOW_MAX_RADIUS, s);
}

fn lines_glow_extent() -> f32 {
    if (uniforms.config.lines_glow_intensity <= 0.001) {
        return 0.0;
    }
    return lines_glow_radius() * LINES_GLOW_EXTENT_MULT;
}

// ---------- Gradient functions (verbatim from lines.wgsl) ----------
fn get_gradient_color_animated(time: f32) -> vec4<f32> {
    let cycle_speed = uniforms.config.lines_animation_speed;
    let t = fract(time * cycle_speed);
    let segments = LINES_PALETTE_SEGMENTS_LOOPED;
    let pos = t * segments;
    let idx = u32(floor(pos)) % LINES_PALETTE_INDEX_MOD;
    let next_idx = (idx + 1u) % LINES_PALETTE_INDEX_MOD;
    let frac = pos - floor(pos);
    let c1 = uniforms.gradient_colors[idx];
    let c2 = uniforms.gradient_colors[next_idx];
    return mix(c1, c2, frac);
}

fn get_gradient_color_static() -> vec4<f32> {
    return uniforms.gradient_colors[0];
}

fn get_gradient_color_by_position(normalized_x: f32) -> vec4<f32> {
    let segments = LINES_PALETTE_SEGMENTS_STATIC;
    let pos = clamp(normalized_x, 0.0, 1.0) * segments;
    let idx = u32(floor(pos));
    let frac = pos - floor(pos);
    if (idx >= LINES_PALETTE_INDEX_TAIL) {
        return uniforms.gradient_colors[LINES_PALETTE_INDEX_TAIL];
    }
    let c1 = uniforms.gradient_colors[idx];
    let c2 = uniforms.gradient_colors[idx + 1u];
    return mix(c1, c2, frac);
}

fn get_gradient_color_by_height(amplitude: f32) -> vec4<f32> {
    let segments = LINES_PALETTE_SEGMENTS_STATIC;
    let pos = clamp(amplitude, 0.0, 1.0) * segments;
    let idx = u32(floor(pos));
    let frac = pos - floor(pos);
    if (idx >= LINES_PALETTE_INDEX_TAIL) {
        return uniforms.gradient_colors[LINES_PALETTE_INDEX_TAIL];
    }
    let c1 = uniforms.gradient_colors[idx];
    let c2 = uniforms.gradient_colors[idx + 1u];
    return mix(c1, c2, frac);
}

fn get_gradient_color_wave(normalized_x: f32, amplitude: f32) -> vec4<f32> {
    let blended = clamp(normalized_x * 0.5 + amplitude * 0.5, 0.0, 1.0);
    let segments = LINES_PALETTE_SEGMENTS_STATIC;
    let pos = blended * segments;
    let idx = u32(floor(pos));
    let frac = pos - floor(pos);
    if (idx >= LINES_PALETTE_INDEX_TAIL) {
        return uniforms.gradient_colors[LINES_PALETTE_INDEX_TAIL];
    }
    let c1 = uniforms.gradient_colors[idx];
    let c2 = uniforms.gradient_colors[idx + 1u];
    return mix(c1, c2, frac);
}

// mode: 0=breathing, 1=static, 2=position, 3=height, 4=gradient(wave)
fn get_lines_gradient_color(time: f32, normalized_x: f32, amplitude: f32) -> vec4<f32> {
    let mode = uniforms.config.lines_gradient_mode;
    if (mode == 1u) {
        return get_gradient_color_static();
    } else if (mode == 2u) {
        return get_gradient_color_by_position(normalized_x);
    } else if (mode == 3u) {
        return get_gradient_color_by_height(amplitude);
    } else if (mode == 4u) {
        return get_gradient_color_wave(normalized_x, amplitude);
    }
    return get_gradient_color_animated(time);
}

fn catmull_rom(p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, p3: vec2<f32>, t: f32) -> vec2<f32> {
    let t2 = t * t;
    let t3 = t2 * t;
    return 0.5 * (
        (2.0 * p1) +
        (-p0 + p2) * t +
        (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2 +
        (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3
    );
}

// ---------- Ring geometry ----------
// Center of the widget in pixels.
fn ring_center() -> vec2<f32> {
    return vec2<f32>(uniforms.viewport.z * 0.5, uniforms.viewport.w * 0.5);
}

// Radius (px) the ring is allowed to reach after reserving stroke + glow margin.
fn ring_available_radius() -> f32 {
    let min_dim = min(uniforms.viewport.z, uniforms.viewport.w);
    let line_thickness = max(uniforms.config.line_thickness, 2.0);
    let margin = line_thickness * 0.5
        + uniforms.config.lines_outline_thickness
        + lines_glow_extent()
        + 2.0;
    let avail = min_dim * 0.5 - margin;
    // Never collapse on a tiny panel (or when glow margin exceeds half the panel).
    return max(avail, min_dim * 0.15);
}

// Cartesian position of control point `i` on the ring. The ANGLE advances with
// the unwrapped index `i` (so the curve sweeps monotonically and the seam joins
// cleanly), while the waveform VALUE is looked up modulo `point_count` (closed
// loop). Returns the position plus the signed deviation (for amplitude/color).
fn ring_point(i: i32, point_count: i32, center: vec2<f32>, base_r: f32, dev_r: f32) -> vec3<f32> {
    let n = point_count;
    let wrapped = ((i % n) + n) % n;
    let raw = clamp(bar_data[u32(wrapped)], -1.0, 1.0);
    let value = clamp(raw * uniforms.config.scope_sensitivity, -1.0, 1.0);

    let angle = (f32(i) / f32(n)) * TAU;
    let radius = base_r + value * dev_r;
    let pos = center + radius * vec2<f32>(cos(angle), sin(angle));
    return vec3<f32>(pos.x, pos.y, value);
}

// ---------- SDF stroke helpers (verbatim from lines.wgsl) ----------
fn sd_segment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-6), 0.0, 1.0);
    return length(pa - ba * h);
}

fn corner_to_quad(corner: u32) -> i32 {
    var idx = array<i32, 6>(0, 1, 2, 2, 1, 3);
    return idx[corner];
}

fn joint_miter(dir1: vec2<f32>, dir2: vec2<f32>, n_ref: vec2<f32>) -> vec3<f32> {
    let n1 = vec2<f32>(-dir1.y, dir1.x);
    let n2 = vec2<f32>(-dir2.y, dir2.x);
    var m = n1 + n2;
    if (length(m) < 1e-4) {
        // Segments nearly anti-parallel: fall back to a butt cap on the ref normal.
        return vec3<f32>(n_ref.x, n_ref.y, 1.0);
    }
    m = normalize(m);
    let factor = 1.0 / max(dot(m, n_ref), 0.1);
    return vec3<f32>(m.x, m.y, factor);
}

// Dense ring-curve point in canvas pixels for a dense ring-sample index
// (SCOPE_SP samples per control-point segment), with the aspect stretch applied.
// Returns (x, y, amplitude): amplitude is recovered from the ISOTROPIC radius
// (before the stretch) so the height/wave gradients still track loudness. The
// closed loop wraps through ring_point (value mod point_count, angle from the
// unwrapped index), so any index — negative or past the end — is valid; floor
// division keeps the (segment, t) decomposition correct for negative indices.
fn scope_dense_point(
    ring_idx: i32,
    point_count: i32,
    center: vec2<f32>,
    base_r: f32,
    dev_r: f32,
    aspect_scale: vec2<f32>,
) -> vec3<f32> {
    let seg = i32(floor(f32(ring_idx) / f32(SCOPE_SP)));
    let samp = ring_idx - seg * SCOPE_SP;
    let t = f32(samp) / f32(SCOPE_SP);

    let p0 = ring_point(seg - 1, point_count, center, base_r, dev_r).xy;
    let p1 = ring_point(seg, point_count, center, base_r, dev_r).xy;
    let p2 = ring_point(seg + 1, point_count, center, base_r, dev_r).xy;
    let p3 = ring_point(seg + 2, point_count, center, base_r, dev_r).xy;

    var cp: vec2<f32>;
    if (uniforms.config.lines_style == 1u) {
        cp = mix(p1, p2, t);
    } else {
        cp = catmull_rom(p0, p1, p2, p3, t);
    }

    let iso_radius = length(cp - center);
    let amp = clamp(abs(iso_radius - base_r) / max(dev_r, 0.0001), 0.0, 1.0);
    let stretched = center + (cp - center) * aspect_scale;
    return vec3<f32>(stretched.x, stretched.y, amp);
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    var output: VertexOutput;

    let point_count = i32(uniforms.config.bar_count);
    if (point_count < 2) {
        return dead_output();
    }

    // Instance 0 = fill, 1 = outline (under), 2 = main line (on top).
    let is_fill_pass = instance_index == 0u;
    let is_outline_pass = instance_index == 1u;

    // Skip the fill pass entirely when fill is disabled.
    if (is_fill_pass && uniforms.config.lines_fill_opacity < 0.001) {
        return dead_output();
    }

    let line_thickness = max(uniforms.config.line_thickness, 2.0);
    let half_thickness = line_thickness * 0.5;
    let outline_extra = uniforms.config.lines_outline_thickness;
    let aa_padding = 1.5;

    // Closed loop: `total_samples` dense segments around the ring (segment i
    // joins dense sample i -> i+1, wrapping back to sample 0 to seal the ring).
    // One miter-tiled quad (6 verts, TriangleList) per segment — adjacent quads
    // share miter corners on the join bisector so they tile with no overlap or
    // gap, and a self-intersecting ribbon fold is impossible.
    let total_samples = point_count * SCOPE_SP;
    let seg_idx = i32(vertex_index / 6u);
    let corner = vertex_index % 6u;
    if (seg_idx >= total_samples) {
        return dead_output();
    }
    let ci = corner_to_quad(corner);

    let center = ring_center();
    let avail = ring_available_radius();
    let radius_frac = clamp(uniforms.config.scope_radius, SCOPE_RADIUS_MIN, SCOPE_RADIUS_MAX);
    let base_r = avail * radius_frac;
    let dev_r = avail * (1.0 - radius_frac);

    let aspect_min_dim = min(uniforms.viewport.z, uniforms.viewport.w);
    let aspect_scale = vec2<f32>(
        uniforms.viewport.z / aspect_min_dim,
        uniforms.viewport.w / aspect_min_dim,
    );

    // Curve points around this ring segment (canvas px, aspect-stretched). The
    // closed loop wraps, so prev/next neighbours are always valid — every joint
    // is a real miter (no caps). dp_*.z carries the (pre-stretch) amplitude.
    let dp_a = scope_dense_point(seg_idx, point_count, center, base_r, dev_r, aspect_scale);
    let dp_b = scope_dense_point(seg_idx + 1, point_count, center, base_r, dev_r, aspect_scale);
    let p_prev = scope_dense_point(seg_idx - 1, point_count, center, base_r, dev_r, aspect_scale).xy;
    let p_a = dp_a.xy;
    let p_b = dp_b.xy;
    let p_next = scope_dense_point(seg_idx + 2, point_count, center, base_r, dev_r, aspect_scale).xy;

    // Per-endpoint gradient color: position around the ring (0..1) + amplitude.
    let nx_a = clamp(f32(seg_idx) / f32(total_samples), 0.0, 1.0);
    let nx_b = clamp(f32(seg_idx + 1) / f32(total_samples), 0.0, 1.0);
    let col_a = get_lines_gradient_color(uniforms.config.time, nx_a, dp_a.z);
    let col_b = get_lines_gradient_color(uniforms.config.time, nx_b, dp_b.z);

    // === FILL PASS: pie slice from the rim to the center, with a radial alpha
    // fade (colored at the rim, transparent at the center). corners 0=rim_a,
    // 2=rim_b, 1/3=center — the quad's two triangles tile into a wedge. ===
    if (is_fill_pass) {
        var fill_point: vec2<f32>;
        var fill_color: vec4<f32>;
        if (ci == 0) {
            fill_point = p_a;
            fill_color = col_a;
            fill_color.a *= uniforms.config.lines_fill_opacity;
        } else if (ci == 2) {
            fill_point = p_b;
            fill_color = col_b;
            fill_color.a *= uniforms.config.lines_fill_opacity;
        } else {
            fill_point = center;
            fill_color = col_a;
            fill_color.a = 0.0;
        }
        let ndc_fill = pixel_to_ndc(fill_point.x, fill_point.y);
        output.position = vec4<f32>(ndc_fill.x, ndc_fill.y, 0.0, 1.0);
        output.color = fill_color;
        output.frag_pos = fill_point;
        output.is_outline = 0.0;
        output.is_fill = 1.0;
        output.seg_a = p_a;
        output.seg_b = p_b;
        return output;
    }

    // === LINE / OUTLINE PASS: miter-tiled quad + SDF coverage ===
    // Tight stroke: core + AA pad only. The glow is the post-process bloom of
    // this crisp ring, so the geometry is NOT widened by the glow extent.
    let actual_half_thickness = select(half_thickness, half_thickness + outline_extra, is_outline_pass);
    let r = actual_half_thickness + aa_padding;

    let ab = p_b - p_a;
    if (length(ab) < 1e-4) {
        return dead_output();
    }
    let dir_ab = normalize(ab);
    let n_ab = vec2<f32>(-dir_ab.y, dir_ab.x);

    var dir_prev = dir_ab;
    let dpv = p_a - p_prev;
    if (length(dpv) > 1e-4) {
        dir_prev = normalize(dpv);
    }
    var dir_next = dir_ab;
    let dnv = p_next - p_b;
    if (length(dnv) > 1e-4) {
        dir_next = normalize(dnv);
    }

    let mA = joint_miter(dir_prev, dir_ab, n_ab);
    let mB = joint_miter(dir_ab, dir_next, n_ab);
    let a_plus = p_a + mA.xy * (r * mA.z);
    let a_minus = p_a - mA.xy * (r * mA.z);
    let b_plus = p_b + mB.xy * (r * mB.z);
    let b_minus = p_b - mB.xy * (r * mB.z);

    var offset_point: vec2<f32>;
    var vert_color: vec4<f32>;
    if (ci == 0) {
        offset_point = a_plus;
        vert_color = col_a;
    } else if (ci == 1) {
        offset_point = a_minus;
        vert_color = col_a;
    } else if (ci == 2) {
        offset_point = b_plus;
        vert_color = col_b;
    } else {
        offset_point = b_minus;
        vert_color = col_b;
    }

    var color: vec4<f32>;
    if (is_outline_pass) {
        var outline_color = uniforms.border_color;
        outline_color.a *= uniforms.config.lines_outline_opacity;
        color = outline_color;
    } else {
        color = vert_color;
    }

    let ndc = pixel_to_ndc(offset_point.x, offset_point.y);
    output.position = vec4<f32>(ndc.x, ndc.y, 0.0, 1.0);
    output.color = color;
    output.frag_pos = offset_point;
    output.is_outline = select(0.0, 1.0, is_outline_pass);
    output.is_fill = 0.0;
    output.seg_a = p_a;
    output.seg_b = p_b;

    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var color = input.color;

    // Fill pass: flat (radial-faded) color, no AA halo.
    if (input.is_fill > 0.5) {
        color.a *= uniforms.config.global_opacity;
        return color;
    }

    let line_thickness = max(uniforms.config.line_thickness, 2.0);
    let half_thickness = line_thickness * 0.5;

    let outline_extra = uniforms.config.lines_outline_thickness;
    let is_outline = input.is_outline > 0.5;
    let actual_half_thickness = select(half_thickness, half_thickness + outline_extra, is_outline);

    // True Euclidean distance to the ring-segment centerline (round joins/caps
    // for free; no fold possible). Crisp core ONLY — the neon halo is the
    // post-process bloom of this thin ring (bloom.wgsl), driven by the scope
    // glow setting, so there is no in-shader halo to spike or facet at sharp
    // radial excursions.
    let dist = sd_segment(input.frag_pos, input.seg_a, input.seg_b);
    let core = smoothstep(actual_half_thickness + 0.75, actual_half_thickness - 0.75, dist);

    color.a *= core;
    color.a *= uniforms.config.global_opacity;

    return color;
}
