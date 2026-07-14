// Visualizer Bars Shader
// GPU-accelerated frequency bar rendering with pixel-perfect sizing
// Based on reference-qml/plugins/cavavisualizer/rendering/barrenderer.cpp
//
// ⚠️  Config struct layout MUST match VisualizerConfig in shader.rs byte-for-byte.
//     If you add/remove/reorder fields, update ALL THREE locations:
//       1. src/widgets/visualizer/shader.rs          (VisualizerConfig)
//       2. src/widgets/visualizer/shaders/bars.wgsl  (Config)
//       3. src/widgets/visualizer/shaders/lines.wgsl (Config)

struct Uniforms {
    viewport: vec4<f32>,  // x, y, width, height in PIXELS
    gradient_colors: array<vec4<f32>, 8>,  // Bar gradient colors (blue to aqua)
    peak_gradient_colors: array<vec4<f32>, 8>,  // Peak breathing colors (warm colors)
    peak_color: vec4<f32>,
    border_color: vec4<f32>,
    config: Config,
    audio: vec4<f32>,  // [beat * reactivity, bass, mid, treble] — appended after config (16-aligned)
}

struct Config {
    bar_count: u32,
    mode: u32,  // 0 = bars, 1 = lines, 2 = scope
    border_width: f32,  // In pixels
    peak_enabled: u32,
    peak_thickness: f32,  // In pixels (e.g., 3.0)
    peak_alpha: f32,
    line_thickness: f32,
    bar_width: f32,      // Fixed bar width in pixels (e.g., 20.0)
    bar_spacing: f32,    // Fixed spacing between bars in pixels (e.g., 2.0)
    edge_spacing: f32,   // Edge spacing for centering bars in pixels
    time: f32,           // Time in seconds for animation
    led_bars: u32,       // 0 = normal bars, 1 = LED segmented bars
    led_segment_height: f32,  // Height of each LED segment in pixels
    led_border_opacity: f32,  // 0.0 = transparent, 1.0 = opaque (border opacity in LED mode)
    border_opacity: f32,      // 0.0 = transparent, 1.0 = opaque (border opacity in non-LED mode)
    gradient_mode: u32,          // 0 = static, 2 = wave (1 is intentionally unused)
    peak_gradient_mode: u32,  // 0=static, 1=cycle, 2=height, 3=match
    peak_mode: u32,           // 0=none, 1=fade, 2=fall, 3=fall_accel, 4=fall_fade
    peak_hold_time: f32,      // Time in seconds for peak to hold
    peak_fade_time: f32,      // Time in seconds for peak to fade (fade mode)
    flash_count: u32,         // Number of bars (for bounds checking)
    bar_depth_3d: f32,        // Isometric 3D depth in pixels (0 = flat)
    gradient_orientation: u32, // 0 = vertical, 1 = horizontal
    average_energy: f32,       // Average bar amplitude (0.0-1.0), computed CPU-side
    global_opacity: f32,       // Overall visualizer opacity (0.0-1.0)
    lines_outline_thickness: f32,  // Lines mode only: outline width in pixels
    lines_outline_opacity: f32,    // Lines mode only: outline alpha (0.0-1.0)
    lines_animation_speed: f32,    // Lines mode only: color cycling speed
    lines_gradient_mode: u32,      // Lines mode only: 0=breathing, 1=static, 2=position, 3=height, 4=gradient
    lines_fill_opacity: f32,       // Lines mode only: fill under curve (0.0 = disabled)
    lines_mirror: u32,             // Lines mode only: 0=normal, 1=mirrored
    lines_glow_intensity: f32,     // Lines mode only: glow bloom (0.0 = disabled)
    lines_style: u32,              // Lines mode only: 0=smooth, 1=angular, 2=stepped
    bars_flash_intensity: f32,     // Bars mode: peak-flash bloom strength (0 = off)
    scope_radius: f32,             // Scope mode: ring radius fraction (unused here)
    scope_sensitivity: f32,        // Scope mode: waveform gain (unused here)
    // Flash intensities: one per bar (0.0-1.0), stored as vec4s
    // Up to 2048 bars = 512 vec4s
    flash_data: array<vec4<f32>, 512>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> bar_data: array<f32>;
@group(0) @binding(2) var<storage, read> peak_data: array<f32>;
@group(0) @binding(3) var<storage, read> peak_alpha_data: array<f32>;

// ---------- Brightness modulation constants ----------
// 3D face brightness: 1.0=front, ~1.4=top, ~0.45=side (set in geometry helpers).
// Thresholds gate the brighten/darken branches; lighten factor scales brighten delta.
const BRIGHTNESS_TOP_THRESHOLD: f32 = 1.1;
const BRIGHTNESS_SIDE_THRESHOLD: f32 = 0.9;
const BRIGHTNESS_LIGHTEN_FACTOR: f32 = 0.5;

// ---------- Animation constants ----------
// Complete cycle through gradient/peak colors in ~4 seconds (1.0 / 0.25).
const BARS_GRADIENT_CYCLE_SPEED: f32 = 0.25;

// Beat lift: brighten the whole bar field by up to this fraction on a full kick
// (uniforms.audio.x = beat_pulse). Keeps the spectrum pumping with the music.
const BARS_BEAT_LIFT: f32 = 0.18;

// ---------- Palette segment-count constants ----------
// CPU side (ThemeBarColors::get_bar_gradient_colors) pads to 8 entries.
// Static gradient uses 6 colors (indices 0..5); looped/animated/peak modes wrap `% 6u`.
// Audit Finding #1: naming only — values are NOT harmonized (HIGH RISK visual regression).
const BARS_PALETTE_SEGMENTS_STATIC: f32 = 5.0;
const BARS_PALETTE_SEGMENTS_LOOPED: f32 = 6.0;
const BARS_PALETTE_INDEX_TAIL: u32 = 5u;
const BARS_PALETTE_INDEX_MOD: u32 = 6u;

// ---------- Shared helpers ----------
// Apply 3D-face brightness modulation: top faces (bm > top threshold) brighten toward white;
// side faces (bm < side threshold) darken multiplicatively; front faces (bm ~= 1.0) pass through.
fn apply_brightness_mod(color: vec4<f32>, bm: f32) -> vec4<f32> {
    if (bm > BRIGHTNESS_TOP_THRESHOLD) {
        let brighten = bm - 1.0;
        return vec4<f32>(
            min(color.r + brighten * BRIGHTNESS_LIGHTEN_FACTOR, 1.0),
            min(color.g + brighten * BRIGHTNESS_LIGHTEN_FACTOR, 1.0),
            min(color.b + brighten * BRIGHTNESS_LIGHTEN_FACTOR, 1.0),
            color.a
        );
    } else if (bm < BRIGHTNESS_SIDE_THRESHOLD) {
        return vec4<f32>(
            color.r * bm,
            color.g * bm,
            color.b * bm,
            color.a
        );
    }
    return color;
}

// Snap bar height to the nearest complete LED segment count, leaving the trailing gap off.
// Returns 0.0 if the height doesn't span a single complete segment.
fn snap_to_led_segments(bar_height: f32, segment_height: f32, segment_gap: f32) -> f32 {
    let segment_period = segment_height + segment_gap;
    let num_complete_segments = floor((bar_height + segment_gap) / segment_period);
    if (num_complete_segments > 0.0) {
        return num_complete_segments * segment_period - segment_gap;
    }
    return 0.0;
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) pixel_y: f32,          // Pixel Y coordinate for gradient calculation
    @location(2) is_gradient_bar: f32,  // 1.0 for gradient bars, 0.0 for borders/peaks
    @location(3) bar_height: f32,       // Bar height in pixels (for LED segment calculations)
    @location(4) bar_index: f32,        // Bar index (for wave calculations)
    @location(5) bar_amplitude: f32,    // Bar amplitude 0-1 (for wave color offset)
    @location(6) average_energy: f32,   // Average energy across all bars (for energy mode)
    @location(7) peak_alpha: f32,       // Peak alpha for fade mode (1.0 = visible, 0.0 = hidden)
    @location(8) brightness_mod: f32,   // 3D face brightness: 1.0=front, 1.4=top, 0.45=side
    @location(9) local_y: f32,          // Y for LED segments (undoes side-face slant)
}

// Convert pixel coordinates to NDC (-1 to 1)
fn pixel_to_ndc(pixel_x: f32, pixel_y: f32) -> vec2<f32> {
    let viewport = uniforms.viewport;
    // Pixel (0,0) is top-left, NDC (-1,1) is top-left
    let ndc_x = (pixel_x / viewport.z) * 2.0 - 1.0;
    let ndc_y = 1.0 - (pixel_y / viewport.w) * 2.0;  // Flip Y: pixel down = NDC down
    return vec2<f32>(ndc_x, ndc_y);
}

// Get gradient color based on normalized height (0.0 = bottom, 1.0 = top)
// Non-looping version - for static gradients where 0=bottom color, 1=top color
fn get_gradient_color(normalized_y: f32) -> vec4<f32> {
    let segments = BARS_PALETTE_SEGMENTS_STATIC;
    let pos = clamp(normalized_y, 0.0, 1.0) * segments;
    let idx = u32(floor(pos));
    let frac = pos - floor(pos);

    if (idx >= BARS_PALETTE_INDEX_TAIL) {
        return uniforms.gradient_colors[BARS_PALETTE_INDEX_TAIL];
    }

    let c1 = uniforms.gradient_colors[idx];
    let c2 = uniforms.gradient_colors[idx + 1u];

    return mix(c1, c2, frac);
}

// Get gradient color with seamless looping (for breathing animations)
// Wraps from color[5] back to color[0] smoothly
fn get_gradient_color_looped(normalized_y: f32) -> vec4<f32> {
    // Use 6 segments so we interpolate through all 6 colors AND back to first
    let segments = BARS_PALETTE_SEGMENTS_LOOPED;
    let pos = fract(normalized_y) * segments;  // fract ensures 0-1 range
    let idx = u32(floor(pos)) % BARS_PALETTE_INDEX_MOD;
    let next_idx = (idx + 1u) % BARS_PALETTE_INDEX_MOD;  // Wrap around to 0 after 5
    let frac = pos - floor(pos);

    let c1 = uniforms.gradient_colors[idx];
    let c2 = uniforms.gradient_colors[next_idx];

    return mix(c1, c2, frac);
}

// Get gradient color based on time (breathing animation cycling through all colors)
fn get_gradient_color_animated(time: f32) -> vec4<f32> {
    // Cycle speed: complete cycle through all colors in ~4 seconds
    let cycle_speed = BARS_GRADIENT_CYCLE_SPEED;  // Lower = slower
    let t = fract(time * cycle_speed);

    // Interpolate through 6 gradient colors (0-5)
    let segments = BARS_PALETTE_SEGMENTS_LOOPED;
    let pos = t * segments;
    let idx = u32(floor(pos)) % BARS_PALETTE_INDEX_MOD;
    let next_idx = (idx + 1u) % BARS_PALETTE_INDEX_MOD;
    let frac = pos - floor(pos);

    let c1 = uniforms.gradient_colors[idx];
    let c2 = uniforms.gradient_colors[next_idx];

    return mix(c1, c2, frac);
}

// Get gradient color with phase offset (for desynchronized breathing effects)
// phase_offset: 0.0-1.0 offset in the color cycle
fn get_gradient_color_animated_offset(time: f32, phase_offset: f32) -> vec4<f32> {
    // Cycle speed: complete cycle through all colors in ~4 seconds
    let cycle_speed = BARS_GRADIENT_CYCLE_SPEED;  // Lower = slower
    let t = fract(time * cycle_speed + phase_offset);

    // Interpolate through 6 gradient colors (0-5)
    let segments = BARS_PALETTE_SEGMENTS_LOOPED;
    let pos = t * segments;
    let idx = u32(floor(pos)) % BARS_PALETTE_INDEX_MOD;
    let next_idx = (idx + 1u) % BARS_PALETTE_INDEX_MOD;
    let frac = pos - floor(pos);

    let c1 = uniforms.gradient_colors[idx];
    let c2 = uniforms.gradient_colors[next_idx];

    return mix(c1, c2, frac);
}

// mode: 0=static (first color), 1=cycle (time cycling), 2=height (position based), 3=match (use bar gradient)
fn get_peak_color(mode: u32, time: f32, normalized_height: f32) -> vec4<f32> {
    if (mode == 0u) {
        // Static: use first peak color only
        return uniforms.peak_gradient_colors[0];
    } else if (mode == 1u) {
        // Cycle: time-based cycling through all peak colors
        let cycle_speed = BARS_GRADIENT_CYCLE_SPEED;  // Complete cycle in ~4 seconds
        let t = fract(time * cycle_speed);

        let segments = BARS_PALETTE_SEGMENTS_LOOPED;
        let pos = t * segments;
        let idx = u32(floor(pos)) % BARS_PALETTE_INDEX_MOD;
        let next_idx = (idx + 1u) % BARS_PALETTE_INDEX_MOD;
        let frac = pos - floor(pos);

        let c1 = uniforms.peak_gradient_colors[idx];
        let c2 = uniforms.peak_gradient_colors[next_idx];

        return mix(c1, c2, frac);
    } else if (mode == 2u) {
        // Height: color based on peak height position (taller peaks = higher colors)
        let segments = BARS_PALETTE_SEGMENTS_STATIC;
        let pos = clamp(normalized_height, 0.0, 1.0) * segments;
        let idx = u32(floor(pos));
        let frac = pos - floor(pos);

        if (idx >= BARS_PALETTE_INDEX_TAIL) {
            return uniforms.peak_gradient_colors[BARS_PALETTE_INDEX_TAIL];
        }

        let c1 = uniforms.peak_gradient_colors[idx];
        let c2 = uniforms.peak_gradient_colors[idx + 1u];

        return mix(c1, c2, frac);
    } else {
        // Match (mode == 3): use bar gradient at peak height position
        let segments = BARS_PALETTE_SEGMENTS_STATIC;
        let pos = clamp(normalized_height, 0.0, 1.0) * segments;
        let idx = u32(floor(pos));
        let frac = pos - floor(pos);

        if (idx >= BARS_PALETTE_INDEX_TAIL) {
            return uniforms.gradient_colors[BARS_PALETTE_INDEX_TAIL];
        }

        let c1 = uniforms.gradient_colors[idx];
        let c2 = uniforms.gradient_colors[idx + 1u];

        return mix(c1, c2, frac);
    }
}

// Get gradient color with breathing animation (height-based gradient with time offset)
// Combines the height-based gradient appearance with time-animated color cycling
// When breathing is disabled, returns a static height-based gradient
fn get_gradient_color_breathing(normalized_y: f32, time: f32, breath_enabled: bool) -> vec4<f32> {
    // If breathing is disabled, use static height-based gradient
    if (!breath_enabled) {
        return get_gradient_color(normalized_y);
    }
    
    // Cycle speed: complete cycle through all colors in ~4 seconds (matching peak animation)
    let cycle_speed = BARS_GRADIENT_CYCLE_SPEED;  // Lower = slower
    let time_offset = fract(time * cycle_speed);

    // Combine height position with time offset for breathing effect
    // Using 6 segments for seamless looping through all colors
    let segments = BARS_PALETTE_SEGMENTS_LOOPED;
    let pos = fract(normalized_y + time_offset) * segments;
    let idx = u32(floor(pos)) % BARS_PALETTE_INDEX_MOD;
    let next_idx = (idx + 1u) % BARS_PALETTE_INDEX_MOD;  // Wrap around to 0 after 5
    let frac = pos - floor(pos);

    let c1 = uniforms.gradient_colors[idx];
    let c2 = uniforms.gradient_colors[next_idx];

    return mix(c1, c2, frac);
}

// Get flash intensity for a specific bar
fn get_flash_intensity(bar_index: u32) -> f32 {
    if (bar_index >= uniforms.config.flash_count) {
        return 0.0;
    }
    
    // Unpack flash intensity from vec4 array
    let vec_idx = bar_index / 4u;
    let component = bar_index % 4u;
    
    if (vec_idx >= 512u) {
        return 0.0;
    }
    
    return uniforms.config.flash_data[vec_idx][component];
}

// Get gradient color with flash effect
// Bars flash towards the opposite gradient color when they hit peaks
fn get_gradient_color_flash(normalized_y: f32, bar_index: u32) -> vec4<f32> {
    // Get base gradient color
    let base_color = get_gradient_color(normalized_y);
    
    // Get flash intensity for this bar
    let flash = get_flash_intensity(bar_index);
    
    if (flash <= 0.01) {
        return base_color;
    }
    
    // Flash effect: lerp towards the opposite end of the gradient
    // This keeps the shimmer colorful and within the user's palette
    let flash_color = get_gradient_color(1.0 - normalized_y);
    return mix(base_color, flash_color, flash * 0.7);  // 70% max flash intensity
}

// Get gradient color with height-based stretching (wave mode)
// Taller bars show more of the bottom gradient colors (gradient stretches)
fn get_gradient_color_stretched(normalized_y: f32, bar_amplitude: f32) -> vec4<f32> {
    // As bar gets taller, stretch the bottom gradient colors
    // This makes the bottom colors occupy more vertical space
    let stretch_factor = 1.0 + (bar_amplitude * 1.5);  // 1.0-2.5 range
    let stretched_y = pow(normalized_y, 1.0 / stretch_factor);
    return get_gradient_color(stretched_y);
}

// Helper: create a "dead" (offscreen) vertex output to cull unused quads
fn dead_output(energy: f32) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(-2.0, -2.0, 0.0, 1.0);
    output.color = vec4<f32>(0.0);
    output.pixel_y = 0.0;
    output.is_gradient_bar = 0.0;
    output.bar_height = 0.0;
    output.bar_index = 0.0;
    output.bar_amplitude = 0.0;
    output.average_energy = energy;
    output.peak_alpha = 0.0;
    output.brightness_mod = 1.0;
    output.local_y = 0.0;
    return output;
}

// ============================================================================
// Geometry helpers — one per face type
// ============================================================================
// Each returns a QuadResult with the 4 corner positions, color, and metadata.
// Shared by both bar and peak paths (they differ only in height/fill_color).

struct QuadResult {
    c_tl: vec2<f32>,
    c_tr: vec2<f32>,
    c_bl: vec2<f32>,
    c_br: vec2<f32>,
    color: vec4<f32>,
    is_gradient_bar: f32,
    brightness: f32,
    quad_h: f32,
}

// Get the border color, applying LED opacity or regular opacity depending on mode
fn get_border_color() -> vec4<f32> {
    var alpha = uniforms.border_color.a;
    if (uniforms.config.led_bars != 0u) {
        alpha *= uniforms.config.led_border_opacity;
    } else {
        alpha *= uniforms.config.border_opacity;
    }
    return vec4<f32>(uniforms.border_color.rgb, alpha);
}

// Front face border: expands on outer silhouette edges (left + bottom).
// In 3D mode, top and right edges are shared with 3D faces — no expansion there.
fn compute_front_border(
    snapped_x: f32, snapped_y: f32,
    bar_width: f32, item_height: f32,
    border_width: f32, depth: f32,
) -> QuadResult {
    var r: QuadResult;
    let bw_top = select(border_width, 0.0, depth > 0.001);
    let bw_right = select(border_width, 0.0, depth > 0.001);
    r.c_tl = vec2<f32>(snapped_x - border_width, snapped_y - bw_top);
    r.c_tr = vec2<f32>(snapped_x + bar_width + bw_right, snapped_y - bw_top);
    r.c_bl = vec2<f32>(snapped_x - border_width, snapped_y + item_height + border_width);
    r.c_br = vec2<f32>(snapped_x + bar_width + bw_right, snapped_y + item_height + border_width);
    r.quad_h = item_height + border_width + bw_top;
    r.color = get_border_color();
    r.is_gradient_bar = 0.0;
    r.brightness = 1.0;
    return r;
}

// Front face fill: the visible bar/peak rectangle.
fn compute_front_fill(
    front_tl: vec2<f32>, front_tr: vec2<f32>,
    front_bl: vec2<f32>, front_br: vec2<f32>,
    item_height: f32, fill_color: vec4<f32>,
    use_gradient: bool,
) -> QuadResult {
    var r: QuadResult;
    r.c_tl = front_tl;
    r.c_tr = front_tr;
    r.c_bl = front_bl;
    r.c_br = front_br;
    r.quad_h = item_height;
    r.color = fill_color;
    r.is_gradient_bar = select(0.0, 1.0, use_gradient);
    r.brightness = 1.0;
    return r;
}

// Top face border: isometric parallelogram border, expands on outer silhouette edges.
// Bottom edge shared with front face — no expansion there.
fn compute_top_border(
    snapped_x: f32, snapped_y: f32,
    bar_width: f32,
    border_width: f32, depth: f32,
) -> QuadResult {
    var r: QuadResult;
    r.c_bl = vec2<f32>(snapped_x - border_width, snapped_y);
    r.c_br = vec2<f32>(snapped_x + bar_width, snapped_y);
    r.c_tl = vec2<f32>(snapped_x + depth - border_width, snapped_y - depth - border_width);
    r.c_tr = vec2<f32>(snapped_x + bar_width + depth + border_width, snapped_y - depth - border_width);
    r.quad_h = depth + 2.0 * border_width;
    r.color = get_border_color();
    r.is_gradient_bar = 0.0;
    r.brightness = 1.0;
    return r;
}

// Top face fill: true isometric parallelogram from front top edge → back top edge.
fn compute_top_fill(
    front_tl: vec2<f32>, front_tr: vec2<f32>,
    depth: f32, fill_color: vec4<f32>,
    use_gradient: bool,
) -> QuadResult {
    var r: QuadResult;
    r.c_bl = front_tl;
    r.c_br = front_tr;
    r.c_tl = vec2<f32>(front_tl.x + depth, front_tl.y - depth);
    r.c_tr = vec2<f32>(front_tr.x + depth, front_tr.y - depth);
    r.quad_h = depth;
    r.color = fill_color;
    r.is_gradient_bar = select(0.0, 1.0, use_gradient);
    r.brightness = 1.4;
    return r;
}

// Side face border: expands on outer silhouette edges.
// Left edge shared with front, top edge shared with top face — no expansion.
fn compute_side_border(
    snapped_x: f32, snapped_y: f32,
    bar_width: f32, item_height: f32,
    border_width: f32, depth: f32,
) -> QuadResult {
    var r: QuadResult;
    r.c_tl = vec2<f32>(snapped_x + bar_width, snapped_y);
    r.c_tr = vec2<f32>(snapped_x + bar_width + depth + border_width, snapped_y - depth - border_width);
    r.c_bl = vec2<f32>(snapped_x + bar_width, snapped_y + item_height + border_width);
    r.c_br = vec2<f32>(snapped_x + bar_width + depth + border_width, snapped_y + item_height - depth + border_width);
    r.quad_h = item_height + 2.0 * border_width;
    r.color = get_border_color();
    r.is_gradient_bar = 0.0;
    r.brightness = 1.0;
    return r;
}

// Side face fill: isometric parallelogram from front right edge → back right edge.
fn compute_side_fill(
    front_tr: vec2<f32>, front_br: vec2<f32>,
    depth: f32, item_height: f32, fill_color: vec4<f32>,
    use_gradient: bool,
) -> QuadResult {
    var r: QuadResult;
    r.c_tl = front_tr;
    r.c_tr = vec2<f32>(front_tr.x + depth, front_tr.y - depth);
    r.c_bl = front_br;
    r.c_br = vec2<f32>(front_br.x + depth, front_br.y - depth);
    r.quad_h = item_height;
    r.color = fill_color;
    r.is_gradient_bar = select(0.0, 1.0, use_gradient);
    r.brightness = 0.45;
    return r;
}

// Dispatch to the correct geometry helper based on face_type.
// face_type: 0=front_border, 1=front_fill, 2=top_border, 3=top_fill, 4=side_border, 5=side_fill
fn compute_face_geometry(
    face_type: u32,
    snapped_x: f32, snapped_y: f32,
    bar_width: f32, item_height: f32,
    border_width: f32, depth: f32,
    front_tl: vec2<f32>, front_tr: vec2<f32>,
    front_bl: vec2<f32>, front_br: vec2<f32>,
    fill_color: vec4<f32>,
    use_gradient: bool,
) -> QuadResult {
    switch (face_type) {
        case 0u: {
            return compute_front_border(snapped_x, snapped_y, bar_width, item_height, border_width, depth);
        }
        case 1u: {
            return compute_front_fill(front_tl, front_tr, front_bl, front_br, item_height, fill_color, use_gradient);
        }
        case 2u: {
            return compute_top_border(snapped_x, snapped_y, bar_width, border_width, depth);
        }
        case 3u: {
            return compute_top_fill(front_tl, front_tr, depth, fill_color, use_gradient);
        }
        case 4u: {
            return compute_side_border(snapped_x, snapped_y, bar_width, item_height, border_width, depth);
        }
        case 5u: {
            return compute_side_fill(front_tr, front_br, depth, item_height, fill_color, use_gradient);
        }
        default: {
            // Should never happen — return front fill as fallback
            return compute_front_fill(front_tl, front_tr, front_bl, front_br, item_height, fill_color, use_gradient);
        }
    }
}

// ============================================================================

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var output: VertexOutput;
    
    let bar_count = uniforms.config.bar_count;
    let border_width = uniforms.config.border_width;
    let peak_enabled = uniforms.config.peak_enabled != 0u;
    let viewport = uniforms.viewport;
    let canvas_width = viewport.z;
    let canvas_height = viewport.w;
    let depth = uniforms.config.bar_depth_3d;
    
    // Read average energy from uniform (computed CPU-side to avoid per-vertex loop)
    let average_energy = uniforms.config.average_energy;
    
    // Reserve space at the top for the peak bar's top border + 3D depth offset
    let top_margin = border_width + depth;
    let usable_height = canvas_height - top_margin;
    
    // Vertices per quad
    let vertices_per_quad = 6u;
    let quad_index = vertex_index / vertices_per_quad;
    let vertex_in_quad = vertex_index % vertices_per_quad;
    
    // Layout (6 quads per bar):
    //   [front_border, front_fill, top_border, top_fill, side_border, side_fill] per bar
    //   then same for peaks
    let quads_per_item = 6u;
    let bar_quads = bar_count * quads_per_item;
    let is_peak = quad_index >= bar_quads;
    
    var bar_idx: u32;
    var face_type: u32;  // 0=front_border, 1=front_fill, 2=top_border, 3=top_fill, 4=side_border, 5=side_fill
    
    if (is_peak) {
        let peak_quad_idx = quad_index - bar_quads;
        bar_idx = peak_quad_idx / quads_per_item;
        face_type = peak_quad_idx % quads_per_item;
        
        if (!peak_enabled || bar_idx >= bar_count) {
            return dead_output(average_energy);
        }
    } else {
        bar_idx = quad_index / quads_per_item;
        face_type = quad_index % quads_per_item;
        
        if (bar_idx >= bar_count) {
            return dead_output(average_energy);
        }
    }
    
    // If 3D is disabled and this is a 3D face (top or side), cull it
    if (depth <= 0.001 && face_type >= 2u) {
        return dead_output(average_energy);
    }
    
    // === Pixel-based bar calculations (matching QML barrenderer.cpp) ===
    let bar_width = uniforms.config.bar_width;
    let bar_spacing = uniforms.config.bar_spacing;
    let edge_spacing = uniforms.config.edge_spacing;
    
    let gap_between_borders = select(0.0, border_width, border_width > 0.0);
    let spacing_per_bar = bar_spacing + gap_between_borders;
    
    let bar_x = edge_spacing + f32(bar_idx) * (bar_width + spacing_per_bar);
    let snapped_x = bar_x;
    
    var c_tl: vec2<f32>;
    var c_tr: vec2<f32>;
    var c_bl: vec2<f32>;
    var c_br: vec2<f32>;
    var color: vec4<f32>;
    var is_gradient_bar: f32 = 0.0;
    var brightness: f32 = 1.0;
    var quad_h: f32 = 0.0;
    
    if (is_peak) {
        let value = peak_data[bar_idx];
        if (value <= 0.001) {
            return dead_output(average_energy);
        }
        
        var peak_thickness: f32;
        if (uniforms.config.led_bars != 0u) {
            peak_thickness = uniforms.config.led_segment_height;
        } else {
            peak_thickness = uniforms.config.bar_width * uniforms.config.peak_thickness;
        }
        
        let clamped_value = clamp(value, 0.0, 1.0);
        var bar_height = clamped_value * usable_height;

        // LED snap
        if (uniforms.config.led_bars != 0u) {
            bar_height = snap_to_led_segments(bar_height, uniforms.config.led_segment_height, uniforms.config.border_width);
        }

        let snapped_bar_height = round(bar_height);
        let snapped_y = top_margin + (usable_height - snapped_bar_height);

        let front_tl = vec2<f32>(snapped_x, snapped_y);
        let front_tr = vec2<f32>(snapped_x + bar_width, snapped_y);
        let front_bl = vec2<f32>(snapped_x, snapped_y + peak_thickness);
        let front_br = vec2<f32>(snapped_x + bar_width, snapped_y + peak_thickness);
        
        let peak_color = get_peak_color(uniforms.config.peak_gradient_mode, uniforms.config.time, clamped_value);
        let fill_color = vec4<f32>(peak_color.rgb, uniforms.config.peak_alpha);
        
        let result = compute_face_geometry(
            face_type, snapped_x, snapped_y, bar_width, peak_thickness,
            border_width, depth,
            front_tl, front_tr, front_bl, front_br,
            fill_color, false
        );
        c_tl = result.c_tl; c_tr = result.c_tr;
        c_bl = result.c_bl; c_br = result.c_br;
        color = result.color;
        is_gradient_bar = result.is_gradient_bar;
        brightness = result.brightness;
        quad_h = result.quad_h;
    } else {
        // Regular bar rendering
        let value = bar_data[bar_idx];
        if (value <= 0.001) {
            return dead_output(average_energy);
        }
        
        let clamped_value = clamp(value, 0.0, 1.0);
        var bar_height = clamped_value * usable_height;

        // LED snap
        if (uniforms.config.led_bars != 0u) {
            bar_height = snap_to_led_segments(bar_height, uniforms.config.led_segment_height, uniforms.config.border_width);
        }

        let snapped_bar_height = round(bar_height);
        let snapped_y = top_margin + (usable_height - snapped_bar_height);

        let front_tl = vec2<f32>(snapped_x, snapped_y);
        let front_tr = vec2<f32>(snapped_x + bar_width, snapped_y);
        let front_bl = vec2<f32>(snapped_x, snapped_y + snapped_bar_height);
        let front_br = vec2<f32>(snapped_x + bar_width, snapped_y + snapped_bar_height);
        
        let fill_color = vec4<f32>(1.0);  // Placeholder — fragment shader computes gradient
        
        let result = compute_face_geometry(
            face_type, snapped_x, snapped_y, bar_width, snapped_bar_height,
            border_width, depth,
            front_tl, front_tr, front_bl, front_br,
            fill_color, true
        );
        c_tl = result.c_tl; c_tr = result.c_tr;
        c_bl = result.c_bl; c_br = result.c_br;
        color = result.color;
        is_gradient_bar = result.is_gradient_bar;
        brightness = result.brightness;
        quad_h = result.quad_h;
    }
    
    
    // Generate quad vertices from 4 corners (2 triangles)
    // Supports both rectangles and parallelograms
    var pixel_pos: vec2<f32>;
    
    switch (vertex_in_quad) {
        case 0u: { pixel_pos = c_tl; }  // top-left
        case 1u: { pixel_pos = c_tr; }  // top-right
        case 2u: { pixel_pos = c_bl; }  // bottom-left
        case 3u: { pixel_pos = c_bl; }  // bottom-left
        case 4u: { pixel_pos = c_tr; }  // top-right
        case 5u: { pixel_pos = c_br; }  // bottom-right
        default: { pixel_pos = c_tl; }
    }
    
    let pixel_x = pixel_pos.x;
    let pixel_y = pixel_pos.y;
    
    // Convert to NDC
    let ndc = pixel_to_ndc(pixel_x, pixel_y);
    
    output.position = vec4<f32>(ndc.x, ndc.y, 0.0, 1.0);
    output.color = color;
    output.pixel_y = pixel_y;
    // For side faces, undo the isometric slant so LED segments align with front face
    if (brightness < 0.5) {
        // Side face: right-edge vertices are offset by -depth in Y
        // Undo that offset so LED gaps stay horizontal
        let depth = uniforms.config.bar_depth_3d;
        switch (vertex_in_quad) {
            case 0u, 2u, 3u: { output.local_y = pixel_y; }          // left edge (front) — no offset
            case 1u, 4u, 5u: { output.local_y = pixel_y + depth; }  // right edge (back) — undo -depth
            default: { output.local_y = pixel_y; }
        }
    } else {
        output.local_y = pixel_y;
    }
    output.is_gradient_bar = is_gradient_bar;
    output.bar_height = quad_h;
    output.bar_index = f32(bar_idx);
    if (is_peak) {
        output.bar_amplitude = clamp(peak_data[bar_idx], 0.0, 1.0);
        output.peak_alpha = peak_alpha_data[bar_idx];
    } else {
        output.bar_amplitude = clamp(bar_data[bar_idx], 0.0, 1.0);
        output.peak_alpha = 1.0;
    }
    output.average_energy = average_energy;
    output.brightness_mod = brightness;
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // For gradient bars, compute color based on Y position (height-based gradient)
    if (input.is_gradient_bar > 0.5) {
        let canvas_height = uniforms.viewport.w;
        let border_width = uniforms.config.border_width;
        let depth = uniforms.config.bar_depth_3d;
        let top_margin = border_width + depth;
        let usable_height = canvas_height - top_margin;
        
        // Normalized Y: 0.0 at bottom (canvas_height), 1.0 at top (top_margin)
        let normalized_y = (canvas_height - input.pixel_y - top_margin) / usable_height;
        let clamped_y = clamp(normalized_y, 0.0, 1.0);
        
        // LED bars mode: create gaps between segments (front face + side face, not top face)
        if (uniforms.config.led_bars != 0u && input.brightness_mod < 1.1) {
            let segment_height = uniforms.config.led_segment_height;
            let segment_gap = uniforms.config.border_width;
            let segment_period = segment_height + segment_gap;
            
            let dist_from_bottom = canvas_height - input.local_y;
            let pos_in_period = dist_from_bottom % segment_period;
            
            if (pos_in_period >= segment_height) {
                discard;
            }
        }
        
        // Determine base gradient position based on orientation
        var base_pos: f32;
        if (uniforms.config.gradient_orientation == 1u) {
            // Horizontal: color based on bar position (left=first, right=last)
            base_pos = clamp(input.bar_index / max(f32(uniforms.config.bar_count) - 1.0, 1.0), 0.0, 1.0);
        } else {
            // Vertical: color based on height within bar
            base_pos = clamped_y;
        }
        
        // Choose color based on gradient mode, using orientation-aware base position.
        //
        // gradient_mode discriminants: 0=static, 2=wave. 1u is intentionally
        // unused — see BarsConfig::get_gradient_mode_value in
        // src/visualizer_config.rs; the bars_gradient_mode_never_emits_dead_1u
        // test pins the emitted set so a future agent who picks 1u as a
        // discriminant fails before shipping a dead branch. (Modes 3–5 —
        // shimmer/energy/alternate — were removed; the glow/bloom/beat effects
        // supersede those per-bar color-cycling gimmicks.)
        let gradient_mode = uniforms.config.gradient_mode;
        var base_color: vec4<f32>;

        if (gradient_mode == 2u) {
            base_color = get_gradient_color_stretched(base_pos, input.bar_amplitude);
        } else {
            // 0u (static) and any unexpected value fall through to the
            // height-based gradient.
            base_color = get_gradient_color(base_pos);
        }
        
        // Peak-flash bloom: bars bloom toward the warm peak color on a peak
        // hit, using the per-bar flash envelope (computed + uploaded every
        // tick, previously unused). Squared for a punchy attack, weighted
        // toward the bar top, additive so it reads as emissive over any
        // gradient mode. Applied before brightness_mod so the 3D top/side
        // faces inherit the bloom and stay shading-consistent.
        let flash = get_flash_intensity(u32(input.bar_index)) * uniforms.config.bars_flash_intensity;
        if (flash > 0.001) {
            let top_weight = 0.4 + 0.6 * clamped_y;
            let bloom = uniforms.peak_gradient_colors[0].rgb * (flash * flash * top_weight);
            base_color = vec4<f32>(min(base_color.rgb + bloom, vec3<f32>(1.0)), base_color.a);
        }

        // Apply 3D brightness modulation
        base_color = apply_brightness_mod(base_color, input.brightness_mod);
        // Front face (bm ~= 1.0): no modification

        // Beat lift: gently brighten the whole spectrum on each kick.
        base_color = vec4<f32>(
            min(base_color.rgb * (1.0 + uniforms.audio.x * BARS_BEAT_LIFT), vec3<f32>(1.0)),
            base_color.a,
        );

        // Apply global opacity
        base_color.a *= uniforms.config.global_opacity;
        return base_color;
    }

    // For borders and peaks, use the color passed from vertex shader
    // Apply brightness modulation for 3D peak faces
    var final_color = input.color;
    final_color = apply_brightness_mod(final_color, input.brightness_mod);

    if (uniforms.config.peak_mode == 1u || uniforms.config.peak_mode == 4u) {
        final_color.a = final_color.a * input.peak_alpha;
    }
    // Apply global opacity
    final_color.a *= uniforms.config.global_opacity;
    return final_color;
}
