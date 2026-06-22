// Visualizer Lines Shader
// GPU-accelerated smooth antialiased line rendering with Catmull-Rom spline interpolation.
//
// The stroke is built as a chain of MITER-TILED per-segment quads (TriangleList):
// each dense spline segment emits one quad whose end-edges meet the neighbouring
// quads exactly on the join bisector, so the quads tile WITHOUT overlap or gap
// and the ribbon can never self-intersect. Coverage is an analytic signed-distance
// field — `sd_segment(frag_pos, seg_a, seg_b)` — so round joins/caps fall out for
// free. This shader draws ONLY the thin crisp stroke; the neon glow is the
// separable-blur BLOOM of this line (bloom.wgsl), which is smooth and eases off
// at sharp tips — an in-shader analytic halo instead spiked or faceted at bends.
// AA technique from: https://www.shadertoy.com/view/4dcfW8 ; sdSegment from IQ
// (https://iquilezles.org/articles/distfunctions2d/).
//
// ⚠️  Config struct layout MUST match VisualizerConfig in shader.rs byte-for-byte.
//     If you add/remove/reorder fields, update ALL THREE locations:
//       1. src/widgets/visualizer/shader.rs          (VisualizerConfig)
//       2. src/widgets/visualizer/shaders/bars.wgsl  (Config)
//       3. src/widgets/visualizer/shaders/lines.wgsl (Config)

struct Uniforms {
    viewport: vec4<f32>,  // x, y, width, height in PIXELS
    gradient_colors: array<vec4<f32>, 8>,  // Expanded to 8 colors for more variety
    peak_gradient_colors: array<vec4<f32>, 8>,  // Peak breathing colors (matches bars.wgsl layout)
    peak_color: vec4<f32>,
    border_color: vec4<f32>,
    config: Config,
    audio: vec4<f32>,  // [beat * reactivity, bass, mid, treble] — appended after config (16-aligned)
}

struct Config {
    bar_count: u32,
    mode: u32,  // 0 = bars, 1 = lines, 2 = scope
    border_width: f32,
    peak_enabled: u32,
    peak_thickness: f32,
    peak_alpha: f32,
    line_thickness: f32,  // Line thickness in pixels
    bar_width: f32,
    bar_spacing: f32,
    edge_spacing: f32,
    time: f32,  // Time in seconds for animation
    led_bars: u32,       // 0 = normal bars, 1 = LED segmented bars (not used in lines mode)
    led_segment_height: f32,  // Height of each LED segment in pixels (not used in lines mode)
    led_border_opacity: f32,  // Border opacity in LED mode (not used in lines mode)
    border_opacity: f32,      // Border opacity in non-LED mode (not used in lines mode)
    gradient_mode: u32,       // 0 = static, 2 = wave (1 is intentionally unused)
    peak_gradient_mode: u32,  // 0=static, 1=cycle, 2=height, 3=match (not used in lines mode)
    peak_mode: u32,           // 0=none, 1=fade, 2=fall, 3=fall_accel, 4=fall_fade (not used in lines mode)
    peak_hold_time: f32,      // Time in seconds for peak to hold (not used in lines mode)
    peak_fade_time: f32,      // Time in seconds for peak to fade (not used in lines mode)
    flash_count: u32,         // Number of bars (not used in lines mode)
    bar_depth_3d: f32,        // Isometric 3D depth in pixels (not used in lines mode)
    gradient_orientation: u32, // 0 = vertical, 1 = horizontal (not used in lines mode)
    average_energy: f32,       // Average bar amplitude (not used in lines mode)
    global_opacity: f32,       // Overall visualizer opacity (0.0-1.0)
    lines_outline_thickness: f32,  // Outline width in pixels (0.0 = disabled)
    lines_outline_opacity: f32,    // Outline alpha (0.0-1.0)
    lines_animation_speed: f32,    // Color cycling speed (0.05-1.0)
    lines_gradient_mode: u32,      // 0=breathing, 1=static, 2=position, 3=height, 4=gradient
    lines_fill_opacity: f32,       // Fill under curve (0.0 = disabled)
    lines_mirror: u32,             // 0=normal, 1=mirrored
    lines_glow_intensity: f32,     // Glow bloom (0.0 = disabled)
    lines_style: u32,              // 0=smooth, 1=angular, 2=stepped
    bars_flash_intensity: f32,     // Bars mode: peak-flash bloom strength (0 = off)
    scope_radius: f32,             // Scope mode: ring radius fraction (not used in lines mode)
    scope_sensitivity: f32,        // Scope mode: waveform gain (not used in lines mode)
    // Flash intensities: one per bar (0.0-1.0), stored as vec4s
    // Up to 2048 bars = 512 vec4s (not used in lines mode)
    flash_data: array<vec4<f32>, 512>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> bar_data: array<f32>;  // Point heights (0.0 - 1.0)
@group(0) @binding(2) var<storage, read> peak_data: array<f32>;  // Not used in lines mode
@group(0) @binding(3) var<storage, read> peak_alpha_data: array<f32>;  // Not used in lines mode

// ---------- Palette segment-count constants ----------
// lines.wgsl interpolates across the full 8-entry palette (matching the CPU pad).
// Static/position/height/wave use 7 segments (indices 0..7); breathing wraps `% 8u`.
// Audit Finding #1: naming only — values are NOT harmonized (HIGH RISK visual regression).
const LINES_PALETTE_SEGMENTS_STATIC: f32 = 7.0;
const LINES_PALETTE_SEGMENTS_LOOPED: f32 = 8.0;
const LINES_PALETTE_INDEX_TAIL: u32 = 7u;
const LINES_PALETTE_INDEX_MOD: u32 = 8u;

// Dense spline samples per data segment. SINGLE source of truth: `vs_main` (the
// per-quad vertex walk) and `dense_point` (the curve decomposition) MUST agree,
// and shader.rs::draw_bars_and_lines hardcodes the matching `16` for the CPU
// draw count (see the "MUST match" comment there).
const LINES_SAMPLES_PER_SEGMENT: i32 = 16;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) frag_pos: vec2<f32>,  // Canvas-space pixel position (interpolated → SDF input)
    @location(2) is_outline: f32,  // 1.0 = outline, 0.0 = main line
    @location(3) is_fill: f32,  // 1.0 = fill pass, 0.0 = line/outline pass
    @location(4) @interpolate(flat) seg_a: vec2<f32>,  // This quad's segment start (canvas px)
    @location(5) @interpolate(flat) seg_b: vec2<f32>,  // This quad's segment end (canvas px)
}

// Helper: create a "dead" (offscreen) vertex output for early-return paths
// (insufficient points, fill disabled, degenerate/out-of-range segment).
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

// Convert pixel coordinates to NDC (-1 to 1)
fn pixel_to_ndc(pixel_x: f32, pixel_y: f32) -> vec2<f32> {
    let viewport = uniforms.viewport;
    let ndc_x = (pixel_x / viewport.z) * 2.0 - 1.0;
    let ndc_y = 1.0 - (pixel_y / viewport.w) * 2.0;
    return vec2<f32>(ndc_x, ndc_y);
}

// ---------- Glow constants/helpers ----------
// The Lines glow is NOT drawn in this shader. The line is rendered as a thin,
// crisp SDF stroke; its neon halo is produced by the separable-blur BLOOM
// post-process (bloom.wgsl), driven from `lines_glow_intensity` (see
// `draw_bars_and_lines` / the bloom wiring in shader.rs). An in-shader analytic
// halo round-capped at sharp vertices (glow spikes) or faceted when made
// anisotropic; a true blur is smooth and eases off at sharp tips. `lines_glow_*`
// survives only to reserve vertical headroom (so the blurred halo has room at
// the canvas edges) — see `get_point` / `dense_point`.
const LINES_GLOW_MIN_RADIUS: f32 = 3.0;
const LINES_GLOW_MAX_RADIUS: f32 = 10.0;
const LINES_GLOW_EXTENT_MULT: f32 = 2.5;

// Halo exp-falloff radius (px) for the current intensity.
fn lines_glow_radius() -> f32 {
    let s = clamp(uniforms.config.lines_glow_intensity, 0.0, 1.0);
    return mix(LINES_GLOW_MIN_RADIUS, LINES_GLOW_MAX_RADIUS, s);
}

// How far (px) to widen the line geometry for the halo. Zero when glow is
// disabled, so a disabled glow costs no extra fragments or headroom.
fn lines_glow_extent() -> f32 {
    if (uniforms.config.lines_glow_intensity <= 0.001) {
        return 0.0;
    }
    return lines_glow_radius() * LINES_GLOW_EXTENT_MULT;
}

// Get gradient color based on time (breathing animation cycling through all colors)
fn get_gradient_color_animated(time: f32) -> vec4<f32> {
    // Cycle speed from config (higher = faster cycling through gradient colors)
    let cycle_speed = uniforms.config.lines_animation_speed;
    let t = fract(time * cycle_speed);

    // Interpolate through 8 gradient colors (0-7)
    let segments = LINES_PALETTE_SEGMENTS_LOOPED;
    let pos = t * segments;
    let idx = u32(floor(pos)) % LINES_PALETTE_INDEX_MOD;
    let next_idx = (idx + 1u) % LINES_PALETTE_INDEX_MOD;
    let frac = pos - floor(pos);

    let c1 = uniforms.gradient_colors[idx];
    let c2 = uniforms.gradient_colors[next_idx];

    return mix(c1, c2, frac);
}

// Static gradient: always use first gradient color
fn get_gradient_color_static() -> vec4<f32> {
    return uniforms.gradient_colors[0];
}

// Position-based gradient: color by horizontal position along the line
// Left = first gradient color, right = last gradient color (bass → treble rainbow)
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

// Height-based gradient: color by amplitude (quiet = bottom colors, loud = top colors)
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

// Wave gradient: blends horizontal position and amplitude for a 2D color field.
// Peaks shift colors further along the palette, creating a music-reactive rainbow.
fn get_gradient_color_wave(normalized_x: f32, amplitude: f32) -> vec4<f32> {
    // Base color from position (0.0-0.5 of palette range)
    // Amplitude pushes further along the palette (0.0-0.5 extra)
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

// Dispatch to the correct gradient function based on lines_gradient_mode
// mode: 0=breathing, 1=static, 2=position, 3=height, 4=gradient
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
    // Default: breathing (mode 0)
    return get_gradient_color_animated(time);
}

// Catmull-Rom spline interpolation
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

// Get point at index, clamping to valid range
fn get_point(idx: i32, point_count: i32, canvas_width: f32, canvas_height: f32) -> vec2<f32> {
    let clamped_idx = clamp(idx, 0, point_count - 1);
    let value = bar_data[u32(clamped_idx)];

    // Clamp value to 0-1 range (CAVA auto-sensitivity can produce values > 1.0)
    let clamped_value = clamp(value, 0.0, 1.0);

    // X position: evenly distributed across width
    let x = f32(clamped_idx) / f32(point_count - 1) * canvas_width;

    // Y position depends on mirror mode
    let line_thickness = max(uniforms.config.line_thickness, 2.0);
    let outline_extra = uniforms.config.lines_outline_thickness;
    let aa_padding = 1.5;
    let max_expansion = ((line_thickness * 0.5) + outline_extra + aa_padding) * 4.0 + lines_glow_extent();

    var y: f32;
    if (uniforms.config.lines_mirror == 1u) {
        // Mirror mode: line extends from center, value controls displacement
        // Center of canvas, with expansion margin
        let center_y = canvas_height * 0.5;
        let drawable_half = (canvas_height - max_expansion * 2.0) * 0.5;
        // Value of 0 = center, value > 0 = extends upward from center
        y = center_y - (clamped_value * drawable_half);
    } else {
        // Normal mode: value maps to height (0 = bottom, 1 = top)
        let drawable_height = canvas_height - max_expansion;
        y = canvas_height - (clamped_value * drawable_height);
    }

    return vec2<f32>(x, y);
}

/// Get the Y coordinate of the fill baseline (bottom for normal, center for mirror)
fn get_fill_baseline(canvas_height: f32) -> f32 {
    if (uniforms.config.lines_mirror == 1u) {
        return canvas_height * 0.5;
    }
    return canvas_height;
}

// ---------- SDF stroke helpers ----------

// Unsigned distance from point `p` to the segment [a, b] (IQ's sdSegment).
// Beyond either endpoint the field becomes distance-to-that-endpoint, i.e. a
// circular cap — which is exactly the round join when two segments share an
// endpoint, so no join geometry or miter math is needed for the *coverage*.
fn sd_segment(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-6), 0.0, 1.0);
    return length(pa - ba * h);
}

// Map a 0..5 triangle-list vertex index onto the four corners of a quad.
// Two triangles, (0,1,2) and (2,1,3), over corners {0,1,2,3}.
fn corner_to_quad(corner: u32) -> i32 {
    var idx = array<i32, 6>(0, 1, 2, 2, 1, 3);
    return idx[corner];
}

// Amplitude (0 silent → 1 loud) recovered from a curve point's Y, matching the
// legacy ribbon's mapping so the height/wave gradients are unchanged.
fn point_amplitude(p: vec2<f32>, canvas_height: f32) -> f32 {
    let line_thickness = max(uniforms.config.line_thickness, 2.0);
    let outline_extra = uniforms.config.lines_outline_thickness;
    let aa_padding = 1.5;
    let max_exp = ((line_thickness * 0.5) + outline_extra + aa_padding) * 4.0 + lines_glow_extent();
    if (uniforms.config.lines_mirror == 1u) {
        let center_y = canvas_height * 0.5;
        return clamp(abs(center_y - p.y) / (center_y - max_exp * 0.5), 0.0, 1.0);
    }
    let draw_h = canvas_height - max_exp;
    return clamp((canvas_height - p.y) / draw_h, 0.0, 1.0);
}

// Curve point in canvas pixels for a dense spline-sample index (16 samples per
// data segment). Includes the post-spline centerline Y-clamp that keeps the
// stroke inside the canvas (matches the legacy ribbon's clamp band). The
// mirror-PASS reflection is applied by the caller, not here.
fn dense_point(dense_idx: i32, point_count: i32, cw: f32, ch: f32) -> vec2<f32> {
    let sps = LINES_SAMPLES_PER_SEGMENT;
    let num_segments = point_count - 1;
    let total = num_segments * sps + 1;
    let di = clamp(dense_idx, 0, total - 1);
    let seg = di / sps;
    let samp = di % sps;
    let t = f32(samp) / f32(sps);

    var cp: vec2<f32>;
    if (di >= total - 1 || seg >= num_segments) {
        cp = get_point(point_count - 1, point_count, cw, ch);
    } else {
        let p0 = get_point(seg - 1, point_count, cw, ch);
        let p1 = get_point(seg,     point_count, cw, ch);
        let p2 = get_point(seg + 1, point_count, cw, ch);
        let p3 = get_point(seg + 2, point_count, cw, ch);
        if (uniforms.config.lines_style == 1u) {
            // Angular: straight segments between data points.
            cp = mix(p1, p2, t);
        } else {
            // Smooth (default): Catmull-Rom spline.
            cp = catmull_rom(p0, p1, p2, p3, t);
        }
    }

    // Post-spline centerline clamp (the legacy ribbon clamped current/prev/next
    // here). Offsets/miters below are deliberately NOT clamped — iced scissors
    // the pass to the clip bounds, so halo overflow is clipped on the GPU.
    let line_thickness = max(uniforms.config.line_thickness, 2.0);
    let outline_extra = uniforms.config.lines_outline_thickness;
    let aa_padding = 1.5;
    let max_expansion = (line_thickness * 0.5) + outline_extra + aa_padding + lines_glow_extent();
    if (uniforms.config.lines_mirror == 1u) {
        let center_y = ch * 0.5;
        cp.y = clamp(cp.y, max_expansion, center_y);
    } else {
        cp.y = clamp(cp.y, max_expansion, ch);
    }
    return cp;
}

// Join geometry: given the unit directions of the two segments meeting at a
// joint (both pointing along the polyline) and the unit normal of the segment
// being extruded, return (miter_normal.xy, length_factor). The corner sits at
// `joint ± miter_normal * R * factor`; because adjacent quads share the same
// miter normal AND factor at a joint, their corners coincide exactly → the
// quads tile with no overlap (no double-composite seam) and no gap (no notch).
// `factor = 1/cos(half-angle)`, floored so a near-180° reversal can't blow up.
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

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32
) -> VertexOutput {
    var output: VertexOutput;

    let point_count = i32(uniforms.config.bar_count);
    let line_thickness = max(uniforms.config.line_thickness, 2.0);
    let half_thickness = line_thickness * 0.5;
    let viewport = uniforms.viewport;
    let canvas_width = viewport.z;
    let canvas_height = viewport.w;

    // Instance layout:
    // 0=fill(top), 1=outline(top), 2=main(top)
    // 3=fill(mirror), 4=outline(mirror), 5=main(mirror)
    let is_mirror_pass = instance_index >= 3u;
    let base_instance = select(instance_index, instance_index - 3u, is_mirror_pass);
    let is_fill_pass = base_instance == 0u;
    let is_outline_pass = base_instance == 1u;
    // base_instance 2 = main line pass

    if (point_count < 2) {
        return dead_output();
    }
    if (is_fill_pass && uniforms.config.lines_fill_opacity < 0.001) {
        return dead_output();
    }

    // Dense polyline: one quad per dense segment (LINES_SAMPLES_PER_SEGMENT per
    // data segment), so `dense_point` and this walk share one divisor.
    let samples_per_segment = LINES_SAMPLES_PER_SEGMENT;
    let num_segments = point_count - 1;
    let num_dense_segments = num_segments * samples_per_segment;

    let seg_idx = i32(vertex_index / 6u);
    let corner = vertex_index % 6u;
    if (seg_idx >= num_dense_segments) {
        return dead_output();
    }
    let ci = corner_to_quad(corner);

    // Curve points around this segment (canvas space), then mirror-pass flip.
    var p_prev = dense_point(seg_idx - 1, point_count, canvas_width, canvas_height);
    var p_a = dense_point(seg_idx, point_count, canvas_width, canvas_height);
    var p_b = dense_point(seg_idx + 1, point_count, canvas_width, canvas_height);
    var p_next = dense_point(seg_idx + 2, point_count, canvas_width, canvas_height);

    if (is_mirror_pass) {
        let cy = canvas_height * 0.5;
        p_prev.y = 2.0 * cy - p_prev.y;
        p_a.y = 2.0 * cy - p_a.y;
        p_b.y = 2.0 * cy - p_b.y;
        p_next.y = 2.0 * cy - p_next.y;
    }

    // Per-endpoint gradient color + amplitude (interpolated across the quad).
    let nx_a = clamp(p_a.x / canvas_width, 0.0, 1.0);
    let nx_b = clamp(p_b.x / canvas_width, 0.0, 1.0);
    let amp_a = point_amplitude(p_a, canvas_height);
    let amp_b = point_amplitude(p_b, canvas_height);
    let col_a = get_lines_gradient_color(uniforms.config.time, nx_a, amp_a);
    let col_b = get_lines_gradient_color(uniforms.config.time, nx_b, amp_b);

    // === FILL PASS: trapezoid from curve to baseline (flat, no SDF) ===
    if (is_fill_pass) {
        let baseline = get_fill_baseline(canvas_height);
        // corners: 0=A_curve, 1=A_base, 2=B_curve, 3=B_base
        var fill_point: vec2<f32>;
        var fill_color: vec4<f32>;
        if (ci == 0) {
            fill_point = p_a;
            fill_color = col_a;
        } else if (ci == 1) {
            fill_point = vec2<f32>(p_a.x, baseline);
            fill_color = col_a;
        } else if (ci == 2) {
            fill_point = p_b;
            fill_color = col_b;
        } else {
            fill_point = vec2<f32>(p_b.x, baseline);
            fill_color = col_b;
        }
        fill_color.a *= uniforms.config.lines_fill_opacity;

        let ndc = pixel_to_ndc(fill_point.x, fill_point.y);
        output.position = vec4<f32>(ndc.x, ndc.y, 0.0, 1.0);
        output.color = fill_color;
        output.frag_pos = fill_point;
        output.is_outline = 0.0;
        output.is_fill = 1.0;
        output.seg_a = p_a;
        output.seg_b = p_b;
        return output;
    }

    // === LINE / OUTLINE PASS: miter-tiled quad + SDF coverage ===
    let outline_extra = uniforms.config.lines_outline_thickness;
    let aa_padding = 1.5;
    // Tight stroke: just the core width + AA pad. The glow is the post-process
    // bloom of this crisp line, so the geometry is NOT widened by the glow extent
    // (widening round-capped into spikes at sharp tips).
    let actual_half_thickness = select(half_thickness, half_thickness + outline_extra, is_outline_pass);
    let r = actual_half_thickness + aa_padding;

    let ab = p_b - p_a;
    if (length(ab) < 1e-4) {
        return dead_output();
    }
    let dir_ab = normalize(ab);
    let n_ab = vec2<f32>(-dir_ab.y, dir_ab.x);

    // Directions into each joint (fall back to the segment's own direction at
    // the polyline's endpoints, giving a butt cap there).
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

    // corners: 0=a_plus, 1=a_minus, 2=b_plus, 3=b_minus
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

    // Fill pass: no antialiasing needed, just use the fill color directly
    if (input.is_fill > 0.5) {
        color.a *= uniforms.config.global_opacity;
        return color;
    }

    // Line/outline pass: antialiased rendering
    let line_thickness = max(uniforms.config.line_thickness, 2.0);
    let half_thickness = line_thickness * 0.5;

    let outline_extra = uniforms.config.lines_outline_thickness;
    let is_outline = input.is_outline > 0.5;
    let actual_half_thickness = select(half_thickness, half_thickness + outline_extra, is_outline);

    // True Euclidean distance to the stroke centerline segment. Because adjacent
    // quads tile on the join bisector, every fragment belongs to exactly one
    // quad, and sd_segment's endpoint behavior rounds the joins for free — a
    // self-intersecting ribbon fold is geometrically impossible here. This is the
    // crisp core ONLY; the neon halo is the post-process bloom of this line
    // (bloom.wgsl), so there is no in-shader glow to spike or facet at bends.
    let dist = sd_segment(input.frag_pos, input.seg_a, input.seg_b);
    let core = smoothstep(actual_half_thickness + 0.75, actual_half_thickness - 0.75, dist);

    color.a *= core;
    color.a *= uniforms.config.global_opacity;

    return color;
}
