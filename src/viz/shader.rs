//! GPU shader-based visualizer — `shader::Program` impl for Firmium.
//!
//! Ported from nokkvi/src/widgets/visualizer/shader.rs with:
//!   - VisualizerState replaced by VizState (Firmium widget-level state)
//!   - ShaderParams replaced by VizConfig
//!   - wgpu 27 API: RenderPassDescriptor has no `multiview_mask` field
//!   - iced 0.14: shader::Event is iced::event::Event

use std::{sync::OnceLock, time::Instant};

use iced::{
    Event, Rectangle, mouse,
    widget::shader::{self, Viewport},
    wgpu,
};

use super::{VizMode, config::VizConfig, pipeline::VisualizerPipeline, state::VizState};

static START_TIME: OnceLock<Instant> = OnceLock::new();

fn get_elapsed_time() -> f32 {
    let start = START_TIME.get_or_init(Instant::now);
    start.elapsed().as_secs_f32()
}

const BLOOM_THRESHOLD: f32 = 0.35;
const LINES_GLOW_BLOOM_GAIN: f32 = 3.2;
const LINES_GLOW_BLUR_ITERATIONS: u32 = 4;
const LINES_GLOW_BLOOM_THRESHOLD: f32 = 0.06;
pub(super) const TRAIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const BLOOM_DIP: f32 = 0.55;
const BLOOM_BEAT_GAIN: f32 = 0.35;
const BLOOM_BASS_GAIN: f32 = 0.85;
const ECHO_MAX_DECAY: f32 = 0.94;
const ECHO_BASE_ZOOM: f32 = 0.012;
const ECHO_BASS_ZOOM: f32 = 0.04;
const ECHO_BASE_ROT: f32 = 0.005;
const ECHO_BEAT_ROT: f32 = 0.02;
const SCOPE_SAMPLES_PER_SEGMENT: u32 = 12;

/// Linearly resamples `data` to exactly `target` points. Used so each mode's
/// independent point/bar-count knob can shrink (or gently interpolate-expand)
/// the backend's fixed-resolution bar/waveform array.
fn resample(data: &[f32], target: usize) -> Vec<f32> {
    let n = data.len();
    if n == 0 || target == 0 {
        return Vec::new();
    }
    if target == n {
        return data.to_vec();
    }
    (0..target)
        .map(|i| {
            let pos = i as f32 / (target - 1).max(1) as f32 * (n - 1) as f32;
            let lo = pos.floor() as usize;
            let hi = (lo + 1).min(n - 1);
            let t = pos - lo as f32;
            data[lo] * (1.0 - t) + data[hi] * t
        })
        .collect()
}

// --- GPU uniform types ---

#[derive(Debug, Clone, Copy)]
#[repr(C, align(16))]
pub(crate) struct VisualizerConfig {
    pub bar_count: u32,
    pub mode: u32,
    pub border_width: f32,
    pub peak_enabled: u32,
    pub peak_thickness: f32,
    pub peak_alpha: f32,
    pub line_thickness: f32,
    pub bar_width: f32,
    pub bar_spacing: f32,
    pub edge_spacing: f32,
    pub time: f32,
    pub led_bars: u32,
    pub led_segment_height: f32,
    pub led_border_opacity: f32,
    pub border_opacity: f32,
    pub gradient_mode: u32,
    pub peak_gradient_mode: u32,
    pub peak_mode: u32,
    pub peak_hold_time: f32,
    pub peak_fade_time: f32,
    pub flash_count: u32,
    pub bar_depth_3d: f32,
    pub gradient_orientation: u32,
    pub average_energy: f32,
    pub global_opacity: f32,
    pub lines_outline_thickness: f32,
    pub lines_outline_opacity: f32,
    pub lines_animation_speed: f32,
    pub lines_gradient_mode: u32,
    pub lines_fill_opacity: f32,
    pub lines_mirror: u32,
    pub lines_glow_intensity: f32,
    pub lines_style: u32,
    pub bars_flash_intensity: f32,
    pub scope_radius: f32,
    pub scope_sensitivity: f32,
    pub flash_data: [[f32; 4]; 512],
}

unsafe impl bytemuck::Pod for VisualizerConfig {}
unsafe impl bytemuck::Zeroable for VisualizerConfig {}

const _: () = assert!(core::mem::align_of::<VisualizerConfig>() == 16);
const _: () = assert!(core::mem::size_of::<VisualizerConfig>() == 8336);
const _: () = assert!(core::mem::offset_of!(VisualizerConfig, flash_data) == 144);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(super) struct BloomParams {
    pub(super) intensity: f32,
    pub(super) threshold: f32,
    pub(super) _pad: [f32; 2],
}
unsafe impl bytemuck::Pod for BloomParams {}
unsafe impl bytemuck::Zeroable for BloomParams {}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(super) struct EchoParams {
    pub(super) decay: f32,
    pub(super) zoom: f32,
    pub(super) sin_a: f32,
    pub(super) cos_a: f32,
}
unsafe impl bytemuck::Pod for EchoParams {}
unsafe impl bytemuck::Zeroable for EchoParams {}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(super) struct CrtParams {
    pub(super) amount: f32,
    pub(super) beat: f32,
    pub(super) time: f32,
    pub(super) _pad: f32,
}
unsafe impl bytemuck::Pod for CrtParams {}
unsafe impl bytemuck::Zeroable for CrtParams {}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(super) struct Uniforms {
    pub(super) viewport: [f32; 4],
    pub(super) gradient_colors: [[f32; 4]; 8],
    pub(super) peak_gradient_colors: [[f32; 4]; 8],
    pub(super) peak_color: [f32; 4],
    pub(super) border_color: [f32; 4],
    pub(super) config: VisualizerConfig,
    pub(super) audio: [f32; 4],
}
unsafe impl bytemuck::Pod for Uniforms {}
unsafe impl bytemuck::Zeroable for Uniforms {}

// --- Primitive ---

#[derive(Debug)]
pub struct VisualizerPrimitive {
    gradient_colors: [[f32; 4]; 8],
    peak_gradient_colors: [[f32; 4]; 8],
    peak_color: [f32; 4],
    border_color: [f32; 4],
    config: VisualizerConfig,
    // Pre-packed data (from draw() time snapshot)
    bar_data: Vec<f32>,
    peak_data: Vec<f32>,
    peak_alpha_data: Vec<f32>,
    audio: [f32; 4],
    particle_data: Vec<[f32; 8]>,
    particle_count: u32,
    has_perspective: bool,
    bloom_enabled: bool,
    bloom_intensity: f32,
    bloom_blur_iterations: u32,
    bloom_threshold: f32,
    beat_reactivity: f32,
    trails_enabled: bool,
    trails_decay: f32,
    echo_enabled: bool,
    echo: f32,
    crt_enabled: bool,
    crt: f32,
    scope_beam: bool,
    // For bloom/echo dynamic computation in prepare()
    beat_pulse: f32,
    bass: f32,
}

impl VisualizerPrimitive {
    pub(crate) fn new(state: &VizState, mode: VizMode, cfg: &VizConfig) -> Self {
        let mut gradient: [[f32; 4]; 8] = [[0.0; 4]; 8];
        for (i, color) in cfg.gradient_colors.iter().take(8).enumerate() {
            gradient[i] = [color.r, color.g, color.b, color.a];
        }

        let mut peak_gradient: [[f32; 4]; 8] = [[0.0; 4]; 8];
        for (i, color) in cfg.peak_gradient_colors.iter().take(8).enumerate() {
            peak_gradient[i] = [color.r, color.g, color.b, color.a];
        }

        let peak_col = [
            cfg.peak_color.r,
            cfg.peak_color.g,
            cfg.peak_color.b,
            cfg.peak_alpha,
        ];
        let border_col = [
            cfg.border_color.r,
            cfg.border_color.g,
            cfg.border_color.b,
            cfg.border_color.a,
        ];

        let flash_intensities = state.get_flash_intensities();
        let flash_count = flash_intensities.len().min(2048) as u32;
        let mut flash_data: [[f32; 4]; 512] = [[0.0; 4]; 512];
        for (i, &intensity) in flash_intensities.iter().take(2048).enumerate() {
            flash_data[i / 4][i % 4] = intensity;
        }

        let is_scope = mode == VizMode::Scope;

        // Scope reuses the Lines uniform slots for outline/gradient/animation/
        // fill/glow/style (only one of Lines/Scope ever renders at a time),
        // selecting its own independent config values instead of Lines'.
        let (
            stroke_outline_thickness,
            stroke_outline_opacity,
            stroke_animation_speed,
            stroke_gradient_mode,
            stroke_fill_opacity,
            stroke_mirror,
            stroke_glow_intensity,
            stroke_style,
        ) = if is_scope {
            (
                cfg.scope_outline_thickness,
                cfg.scope_outline_opacity,
                cfg.scope_animation_speed,
                cfg.scope_gradient_mode,
                cfg.scope_fill_opacity,
                false,
                cfg.scope_glow_intensity,
                cfg.scope_style,
            )
        } else {
            (
                cfg.lines_outline_thickness,
                cfg.lines_outline_opacity,
                cfg.lines_animation_speed,
                cfg.lines_gradient_mode,
                cfg.lines_fill_opacity,
                cfg.lines_mirror,
                cfg.lines_glow_intensity,
                cfg.lines_style,
            )
        };

        // Each mode's motion-trail/echo knob is independent.
        let (mode_trails, mode_echo) = match mode {
            VizMode::Bars => (cfg.bars_trails, cfg.bars_echo),
            VizMode::Lines => (cfg.lines_trails, cfg.lines_echo),
            VizMode::Scope => (cfg.scope_trails, cfg.scope_echo),
        };

        // Point/bar count is independent per mode; the backend always
        // produces a fixed BAR_COUNT-length array, so a lower target count
        // downsamples it (a higher one just interpolates — no extra detail
        // beyond the backend's native resolution).
        let target_points = match mode {
            VizMode::Bars => cfg.bars_max_bars,
            VizMode::Lines => cfg.lines_point_count,
            VizMode::Scope => cfg.scope_point_count,
        } as usize;

        let bars = resample(&state.get_bars(), target_points);
        let bar_count_val = bars.len();
        let average_energy = if bar_count_val > 0 {
            bars.iter().sum::<f32>() / bar_count_val as f32
        } else {
            0.0
        };

        let viz_config = VisualizerConfig {
            bar_count: bar_count_val as u32,
            mode: match mode {
                VizMode::Bars => 0,
                VizMode::Lines => 1,
                VizMode::Scope => 2,
            },
            border_width: cfg.border_width,
            peak_enabled: u32::from(cfg.peak_enabled),
            peak_thickness: cfg.peak_thickness,
            peak_alpha: cfg.peak_alpha,
            line_thickness: cfg.line_thickness,
            bar_width: cfg.bar_width,
            bar_spacing: cfg.bar_spacing,
            edge_spacing: cfg.edge_spacing,
            time: get_elapsed_time(),
            led_bars: u32::from(cfg.led_bars),
            led_segment_height: cfg.led_segment_height,
            led_border_opacity: cfg.led_border_opacity,
            border_opacity: cfg.border_opacity,
            gradient_mode: cfg.gradient_mode,
            peak_gradient_mode: cfg.peak_gradient_mode,
            peak_mode: cfg.peak_mode,
            peak_hold_time: cfg.peak_hold_time,
            peak_fade_time: cfg.peak_fade_time,
            flash_count,
            bar_depth_3d: cfg.bar_depth_3d,
            gradient_orientation: cfg.gradient_orientation,
            average_energy,
            global_opacity: cfg.global_opacity,
            lines_outline_thickness: stroke_outline_thickness,
            lines_outline_opacity: stroke_outline_opacity,
            lines_animation_speed: stroke_animation_speed,
            lines_gradient_mode: stroke_gradient_mode,
            lines_fill_opacity: stroke_fill_opacity,
            lines_mirror: u32::from(stroke_mirror),
            lines_glow_intensity: stroke_glow_intensity,
            lines_style: stroke_style,
            bars_flash_intensity: cfg.bars_flash_intensity,
            scope_radius: cfg.scope_radius,
            scope_sensitivity: cfg.scope_sensitivity,
            flash_data,
        };

        let has_perspective = cfg.bar_depth_3d > 0.001;
        let effect_bloom = cfg.bloom_enabled && cfg.bloom_intensity > 0.001;
        let stroke_glow = matches!(mode, VizMode::Lines | VizMode::Scope)
            && stroke_glow_intensity > 0.001;
        let stroke_bloom_intensity = if stroke_glow {
            LINES_GLOW_BLOOM_GAIN * stroke_glow_intensity
        } else {
            0.0
        };
        let bloom_enabled = effect_bloom || stroke_glow;
        let bloom_intensity = if effect_bloom {
            cfg.bloom_intensity.max(stroke_bloom_intensity)
        } else {
            stroke_bloom_intensity
        };
        let bloom_blur_iterations =
            if stroke_glow { LINES_GLOW_BLUR_ITERATIONS } else { 1 };
        let bloom_threshold =
            if stroke_glow { LINES_GLOW_BLOOM_THRESHOLD } else { BLOOM_THRESHOLD };

        let trails_enabled = mode_trails > 0.001;
        let trails_decay = if trails_enabled {
            0.6 + 0.32 * mode_trails.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let echo_enabled = mode_echo > 0.001;
        let crt_enabled = cfg.crt > 0.001;

        let particle_data = if mode == VizMode::Scope && cfg.scope_particles {
            let opacity = cfg.global_opacity.clamp(0.0, 1.0);
            let mut p = state.get_particles();
            p.truncate(VisualizerPipeline::MAX_PARTICLES);
            for particle in &mut p {
                particle[3] *= opacity;
            }
            p
        } else {
            Vec::new()
        };
        let particle_count = particle_data.len() as u32;

        // Snapshot bar/peak/waveform data
        let bar_data: Vec<f32> = if viz_config.mode == 2 {
            resample(&state.get_waveform(), target_points)
        } else {
            bars
        };
        let peak_data: Vec<f32> = resample(&state.get_peak_bars(), target_points);
        let peak_alpha_data: Vec<f32> = resample(&state.get_peak_alphas(), target_points);

        let (bass, mid, treble) = state.current_bands();
        let beat = state.current_beat_pulse() * cfg.beat_reactivity;
        let audio = [beat, bass, mid, treble];

        Self {
            gradient_colors: gradient,
            peak_gradient_colors: peak_gradient,
            peak_color: peak_col,
            border_color: border_col,
            config: viz_config,
            bar_data,
            peak_data,
            peak_alpha_data,
            audio,
            particle_data,
            particle_count,
            has_perspective,
            bloom_enabled,
            bloom_intensity,
            bloom_blur_iterations,
            bloom_threshold,
            beat_reactivity: cfg.beat_reactivity,
            trails_enabled,
            trails_decay,
            echo_enabled,
            echo: mode_echo,
            crt_enabled,
            crt: cfg.crt,
            scope_beam: cfg.scope_beam,
            beat_pulse: state.current_beat_pulse(),
            bass,
        }
    }

    fn draw_bars_and_lines(
        config: &VisualizerConfig,
        bind_group: &wgpu::BindGroup,
        bars_pipeline: &wgpu::RenderPipeline,
        lines_pipeline: &wgpu::RenderPipeline,
        scope_pipeline: &wgpu::RenderPipeline,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) {
        let bar_count = config.bar_count;
        render_pass.set_bind_group(0, bind_group, &[]);

        match config.mode {
            0 => {
                render_pass.set_pipeline(bars_pipeline);
                let vertices_per_bar = 6;
                let quads_per_bar = 6u32;
                let peak_multiplier =
                    if config.peak_enabled != 0 { quads_per_bar } else { 0 };
                let total_quads = bar_count * quads_per_bar + bar_count * peak_multiplier;
                render_pass.draw(0..(total_quads * vertices_per_bar), 0..1);
            }
            1 => {
                render_pass.set_pipeline(lines_pipeline);
                let samples_per_segment = 16u32;
                let num_segments = bar_count.saturating_sub(1);
                let num_dense_segments = num_segments * samples_per_segment;
                let vertices_per_quad = 6u32;
                let vertices_per_pass = num_dense_segments * vertices_per_quad;
                let instance_count = if config.lines_mirror != 0 { 6 } else { 3 };
                render_pass.draw(0..vertices_per_pass, 0..instance_count);
            }
            2 => {
                render_pass.set_pipeline(scope_pipeline);
                let samples_per_segment = SCOPE_SAMPLES_PER_SEGMENT;
                let total_samples = bar_count * samples_per_segment;
                let vertices_per_quad = 6u32;
                let vertices_per_pass = total_samples * vertices_per_quad;
                render_pass.draw(0..vertices_per_pass, 0..3);
            }
            _ => {}
        }
    }

    fn draw_particles(
        particle_pipeline: &wgpu::RenderPipeline,
        bind_group: &wgpu::BindGroup,
        particle_count: u32,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) {
        if particle_count == 0 {
            return;
        }
        render_pass.set_pipeline(particle_pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..6, 0..particle_count);
    }

    fn render_without_msaa(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
        pipeline: &VisualizerPipeline,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("visualizer non-MSAA render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_viewport(
            clip_bounds.x as f32,
            clip_bounds.y as f32,
            clip_bounds.width as f32,
            clip_bounds.height as f32,
            0.0,
            1.0,
        );
        render_pass.set_scissor_rect(
            clip_bounds.x,
            clip_bounds.y,
            clip_bounds.width,
            clip_bounds.height,
        );

        let scope_pipe = if self.scope_beam {
            &pipeline.scope_pipeline_beam
        } else {
            &pipeline.scope_pipeline
        };
        Self::draw_bars_and_lines(
            &self.config,
            &pipeline.bind_group,
            &pipeline.bars_pipeline,
            &pipeline.lines_pipeline,
            scope_pipe,
            &mut render_pass,
        );
        Self::draw_particles(
            &pipeline.particle_pipeline,
            &pipeline.bind_group,
            self.particle_count,
            &mut render_pass,
        );
    }
}

impl shader::Primitive for VisualizerPrimitive {
    type Pipeline = VisualizerPipeline;

    fn prepare(
        &self,
        pipeline: &mut Self::Pipeline,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bounds: &Rectangle,
        viewport: &Viewport,
    ) {
        let uniforms = Uniforms {
            viewport: [bounds.x, bounds.y, bounds.width, bounds.height],
            gradient_colors: self.gradient_colors,
            peak_gradient_colors: self.peak_gradient_colors,
            peak_color: self.peak_color,
            border_color: self.border_color,
            config: self.config,
            audio: self.audio,
        };
        queue.write_buffer(
            &pipeline.uniform_buffer,
            0,
            bytemuck::bytes_of(&uniforms),
        );

        let mut bar_data_padded = self.bar_data.clone();
        bar_data_padded.resize(pipeline.max_bars, 0.0);
        queue.write_buffer(
            &pipeline.bar_buffer,
            0,
            bytemuck::cast_slice(&bar_data_padded),
        );

        let mut peak_data_padded = self.peak_data.clone();
        peak_data_padded.resize(pipeline.max_bars, 0.0);
        queue.write_buffer(
            &pipeline.peak_buffer,
            0,
            bytemuck::cast_slice(&peak_data_padded),
        );

        let mut peak_alpha_padded = self.peak_alpha_data.clone();
        peak_alpha_padded.resize(pipeline.max_bars, 1.0);
        queue.write_buffer(
            &pipeline.peak_alpha_buffer,
            0,
            bytemuck::cast_slice(&peak_alpha_padded),
        );

        if !self.particle_data.is_empty() {
            queue.write_buffer(
                &pipeline.particle_buffer,
                0,
                bytemuck::cast_slice(&self.particle_data),
            );
        }

        if self.has_perspective
            || self.bloom_enabled
            || self.trails_enabled
            || self.echo_enabled
            || self.crt_enabled
        {
            let scale = viewport.scale_factor();
            let w = (bounds.width * scale).ceil() as u32;
            let h = (bounds.height * scale).ceil() as u32;

            if w > 0 && h > 0 && pipeline.msaa_size != (w, h) {
                let msaa_tex = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("visualizer MSAA texture"),
                    size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 4,
                    dimension: wgpu::TextureDimension::D2,
                    format: pipeline.format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                });
                let msaa_view = msaa_tex.create_view(&wgpu::TextureViewDescriptor::default());

                let resolve_tex = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("visualizer resolve texture"),
                    size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: pipeline.format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                let resolve_view =
                    resolve_tex.create_view(&wgpu::TextureViewDescriptor::default());

                let ring_only_tex = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("visualizer ring-only resolve texture"),
                    size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: pipeline.format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                let ring_only_view =
                    ring_only_tex.create_view(&wgpu::TextureViewDescriptor::default());

                let blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("visualizer blit bind group"),
                    layout: &pipeline.blit_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&resolve_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                        },
                    ],
                });

                let bw = (w / 2).max(1);
                let bh = (h / 2).max(1);
                let bloom_desc = wgpu::TextureDescriptor {
                    label: Some("visualizer bloom texture"),
                    size: wgpu::Extent3d {
                        width: bw,
                        height: bh,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: pipeline.format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                };
                let bloom_a = device.create_texture(&bloom_desc);
                let bloom_a_view = bloom_a.create_view(&wgpu::TextureViewDescriptor::default());
                let bloom_b = device.create_texture(&bloom_desc);
                let bloom_b_view = bloom_b.create_view(&wgpu::TextureViewDescriptor::default());

                let mk_bloom_bg = |view: &wgpu::TextureView| {
                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("visualizer bloom bg"),
                        layout: &pipeline.bloom_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: pipeline.bloom_uniform_buffer.as_entire_binding(),
                            },
                        ],
                    })
                };
                let bloom_bg_scene = mk_bloom_bg(&resolve_view);
                let bloom_bg_a = mk_bloom_bg(&bloom_a_view);
                let bloom_bg_b = mk_bloom_bg(&bloom_b_view);

                let trail_tex = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("visualizer trail texture"),
                    size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: TRAIL_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                let trail_view = trail_tex.create_view(&wgpu::TextureViewDescriptor::default());
                let blit_bg_trail = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("visualizer trail blit bind group"),
                    layout: &pipeline.blit_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&trail_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                        },
                    ],
                });

                let echo_size = wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 };
                let echo_tex = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("visualizer echo texture"),
                    size: echo_size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: TRAIL_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
                let echo_tex_view =
                    echo_tex.create_view(&wgpu::TextureViewDescriptor::default());
                let echo_temp_tex = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("visualizer echo scratch"),
                    size: echo_size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: TRAIL_FORMAT,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                let echo_temp_view =
                    echo_temp_tex.create_view(&wgpu::TextureViewDescriptor::default());
                let echo_feedback_bg =
                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("visualizer echo feedback bind group"),
                        layout: &pipeline.echo_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(
                                    &ring_only_view,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::TextureView(
                                    &echo_temp_view,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: pipeline.echo_uniform_buffer.as_entire_binding(),
                            },
                        ],
                    });
                let blit_bg_echo = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("visualizer echo blit bind group"),
                    layout: &pipeline.blit_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&echo_tex_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
                        },
                    ],
                });

                pipeline.msaa_texture = Some((msaa_tex, msaa_view));
                pipeline.resolve_texture = Some((resolve_tex, resolve_view));
                pipeline.ring_only_texture = Some((ring_only_tex, ring_only_view));
                pipeline.blit_bind_group = Some(blit_bind_group);
                pipeline.bloom_texture_a = Some((bloom_a, bloom_a_view));
                pipeline.bloom_texture_b = Some((bloom_b, bloom_b_view));
                pipeline.bloom_bg_scene = Some(bloom_bg_scene);
                pipeline.bloom_bg_a = Some(bloom_bg_a);
                pipeline.bloom_bg_b = Some(bloom_bg_b);
                pipeline.trail_texture = Some((trail_tex, trail_view));
                pipeline.blit_bg_trail = Some(blit_bg_trail);
                pipeline.echo_texture = Some((echo_tex, echo_tex_view));
                pipeline.echo_temp = Some((echo_temp_tex, echo_temp_view));
                pipeline.echo_feedback_bg = Some(echo_feedback_bg);
                pipeline.blit_bg_echo = Some(blit_bg_echo);
                pipeline.msaa_size = (w, h);
            }
        }

        if self.bloom_enabled {
            let dip_base = 1.0 - self.beat_reactivity * (1.0 - BLOOM_DIP);
            let pump = (BLOOM_BEAT_GAIN * self.beat_pulse + BLOOM_BASS_GAIN * self.bass)
                * self.beat_reactivity;
            let intensity = self.bloom_intensity * (dip_base + pump);
            let bloom_params = BloomParams {
                intensity,
                threshold: self.bloom_threshold,
                _pad: [0.0; 2],
            };
            queue.write_buffer(
                &pipeline.bloom_uniform_buffer,
                0,
                bytemuck::bytes_of(&bloom_params),
            );
        }

        pipeline.trail_needs_clear = self.trails_enabled && !pipeline.trails_were_active;
        pipeline.trails_were_active = self.trails_enabled;

        pipeline.echo_needs_clear = self.echo_enabled && !pipeline.echo_were_active;
        pipeline.echo_were_active = self.echo_enabled;
        if self.echo_enabled {
            let decay = if pipeline.echo_needs_clear {
                0.0
            } else {
                self.echo * ECHO_MAX_DECAY
            };
            let zoom = 1.0
                + ECHO_BASE_ZOOM
                + self.bass * ECHO_BASS_ZOOM * self.beat_reactivity;
            let angle = ECHO_BASE_ROT + self.beat_pulse * ECHO_BEAT_ROT * self.beat_reactivity;
            let (sin_a, cos_a) = angle.sin_cos();
            let echo_params = EchoParams { decay, zoom, sin_a, cos_a };
            queue.write_buffer(
                &pipeline.echo_uniform_buffer,
                0,
                bytemuck::bytes_of(&echo_params),
            );
        }

        if self.crt_enabled {
            let crt_params = CrtParams {
                amount: self.crt,
                beat: self.beat_pulse,
                time: get_elapsed_time(),
                _pad: 0.0,
            };
            queue.write_buffer(
                &pipeline.crt_uniform_buffer,
                0,
                bytemuck::bytes_of(&crt_params),
            );
        }
    }

    fn draw(
        &self,
        pipeline: &Self::Pipeline,
        render_pass: &mut wgpu::RenderPass<'_>,
    ) -> bool {
        if self.config.bar_count == 0 {
            return true;
        }
        if self.has_perspective
            || self.bloom_enabled
            || self.trails_enabled
            || self.echo_enabled
            || self.crt_enabled
        {
            return false;
        }
        let scope_pipe = if self.scope_beam {
            &pipeline.scope_pipeline_beam
        } else {
            &pipeline.scope_pipeline
        };
        Self::draw_bars_and_lines(
            &self.config,
            &pipeline.bind_group,
            &pipeline.bars_pipeline,
            &pipeline.lines_pipeline,
            scope_pipe,
            render_pass,
        );
        Self::draw_particles(
            &pipeline.particle_pipeline,
            &pipeline.bind_group,
            self.particle_count,
            render_pass,
        );
        true
    }

    fn render(
        &self,
        pipeline: &Self::Pipeline,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clip_bounds: &Rectangle<u32>,
    ) {
        if self.config.bar_count == 0 {
            return;
        }
        let width = clip_bounds.width;
        let height = clip_bounds.height;
        if width == 0 || height == 0 {
            return;
        }

        let (msaa_view, resolve_view, blit_bg) = match (
            &pipeline.msaa_texture,
            &pipeline.resolve_texture,
            &pipeline.blit_bind_group,
        ) {
            (Some((_, mv)), Some((_, rv)), Some(bg)) => (mv, rv, bg),
            _ => return self.render_without_msaa(encoder, target, clip_bounds, pipeline),
        };

        // Pass 1: Render into MSAA texture, resolve into resolve texture.
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("visualizer MSAA render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: msaa_view,
                    depth_slice: None,
                    resolve_target: Some(resolve_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let (tex_w, tex_h) = pipeline.msaa_size;
            render_pass.set_viewport(0.0, 0.0, tex_w as f32, tex_h as f32, 0.0, 1.0);
            render_pass.set_scissor_rect(0, 0, tex_w, tex_h);
            let scope_pipe = if self.scope_beam {
                &pipeline.scope_pipeline_beam_msaa
            } else {
                &pipeline.scope_pipeline_msaa
            };
            Self::draw_bars_and_lines(
                &self.config,
                &pipeline.bind_group,
                &pipeline.bars_pipeline_msaa,
                &pipeline.lines_pipeline_msaa,
                scope_pipe,
                &mut render_pass,
            );
            Self::draw_particles(
                &pipeline.particle_pipeline_msaa,
                &pipeline.bind_group,
                self.particle_count,
                &mut render_pass,
            );
        }

        // Pass 1b (echo): render ring only (no particles) into ring_only_texture.
        if self.echo_enabled {
            if let Some((_, ring_only_view)) = &pipeline.ring_only_texture {
                let mut ring_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("visualizer ring-only (echo scene) pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: msaa_view,
                        depth_slice: None,
                        resolve_target: Some(ring_only_view),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Discard,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                let (tex_w, tex_h) = pipeline.msaa_size;
                ring_pass.set_viewport(0.0, 0.0, tex_w as f32, tex_h as f32, 0.0, 1.0);
                ring_pass.set_scissor_rect(0, 0, tex_w, tex_h);
                let scope_pipe = if self.scope_beam {
                    &pipeline.scope_pipeline_beam_msaa
                } else {
                    &pipeline.scope_pipeline_msaa
                };
                Self::draw_bars_and_lines(
                    &self.config,
                    &pipeline.bind_group,
                    &pipeline.bars_pipeline_msaa,
                    &pipeline.lines_pipeline_msaa,
                    scope_pipe,
                    &mut ring_pass,
                );
            }
        }

        // Bloom passes.
        let bloom_views = match (
            self.bloom_enabled,
            &pipeline.bloom_texture_a,
            &pipeline.bloom_texture_b,
            &pipeline.bloom_bg_scene,
            &pipeline.bloom_bg_a,
            &pipeline.bloom_bg_b,
        ) {
            (true, Some((_, av)), Some((_, bv)), Some(bgs), Some(bga), Some(bgb)) => {
                Some((av, bv, bgs, bga, bgb))
            }
            _ => None,
        };

        if let Some((bloom_a_view, bloom_b_view, bg_scene, bg_a, bg_b)) = bloom_views {
            let bw = (pipeline.msaa_size.0 / 2).max(1);
            let bh = (pipeline.msaa_size.1 / 2).max(1);
            let total_passes = 2 * self.bloom_blur_iterations;
            for pass_i in 0..total_passes {
                let (label, view, blur_pipeline, bind_group) = if pass_i == 0 {
                    (
                        "visualizer bloom bright/H pass",
                        bloom_a_view,
                        &pipeline.bloom_bright_pipeline,
                        bg_scene,
                    )
                } else if pass_i % 2 == 1 {
                    (
                        "visualizer bloom blur V pass",
                        bloom_b_view,
                        &pipeline.bloom_blur_v_pipeline,
                        bg_a,
                    )
                } else {
                    (
                        "visualizer bloom blur H pass",
                        bloom_a_view,
                        &pipeline.bloom_blur_h_pipeline,
                        bg_b,
                    )
                };
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(label),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_viewport(0.0, 0.0, bw as f32, bh as f32, 0.0, 1.0);
                pass.set_scissor_rect(0, 0, bw, bh);
                pass.set_pipeline(blur_pipeline);
                pass.set_bind_group(0, bind_group, &[]);
                pass.draw(0..3, 0..1);
            }
        }

        // Motion trails.
        let trail_handles = match (
            self.trails_enabled,
            &pipeline.trail_texture,
            &pipeline.blit_bg_trail,
        ) {
            (true, Some((_, tv)), Some(bg)) => Some((tv, bg)),
            _ => None,
        };

        if let Some((trail_view, _)) = trail_handles {
            let (tex_w, tex_h) = pipeline.msaa_size;
            let decay = self.trails_decay as f64;
            let fade_load = if pipeline.trail_needs_clear {
                wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
            } else {
                wgpu::LoadOp::Load
            };
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("visualizer trail fade pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: trail_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: fade_load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_viewport(0.0, 0.0, tex_w as f32, tex_h as f32, 0.0, 1.0);
                pass.set_scissor_rect(0, 0, tex_w, tex_h);
                pass.set_blend_constant(wgpu::Color {
                    r: decay, g: decay, b: decay, a: decay,
                });
                pass.set_pipeline(&pipeline.trail_fade_pipeline);
                pass.set_bind_group(0, blit_bg, &[]);
                pass.draw(0..3, 0..1);
            }
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("visualizer trail max pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: trail_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_viewport(0.0, 0.0, tex_w as f32, tex_h as f32, 0.0, 1.0);
                pass.set_scissor_rect(0, 0, tex_w, tex_h);
                pass.set_pipeline(&pipeline.trail_max_pipeline);
                pass.set_bind_group(0, blit_bg, &[]);
                pass.draw(0..3, 0..1);
            }
        }

        // Echo (Milkdrop feedback).
        let echo_handles = match (
            self.echo_enabled,
            &pipeline.echo_texture,
            &pipeline.echo_temp,
            &pipeline.echo_feedback_bg,
            &pipeline.blit_bg_echo,
        ) {
            (true, Some((etex, ev)), Some((ttex, _)), Some(fbg), Some(dbg)) => {
                Some((etex, ev, ttex, fbg, dbg))
            }
            _ => None,
        };

        if let Some((echo_tex, echo_view, echo_temp_tex, feedback_bg, _)) = echo_handles {
            let (tex_w, tex_h) = pipeline.msaa_size;
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: echo_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: echo_temp_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: tex_w,
                    height: tex_h,
                    depth_or_array_layers: 1,
                },
            );
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("visualizer echo feedback pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: echo_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_viewport(0.0, 0.0, tex_w as f32, tex_h as f32, 0.0, 1.0);
            pass.set_scissor_rect(0, 0, tex_w, tex_h);
            pass.set_pipeline(&pipeline.echo_feedback_pipeline);
            pass.set_bind_group(0, feedback_bg, &[]);
            pass.draw(0..3, 0..1);
        }

        let display_bg = match echo_handles {
            Some((.., dbg)) => dbg,
            None => match trail_handles {
                Some((_, bg)) => bg,
                None => blit_bg,
            },
        };

        // Pass 2: Blit the scene onto the framebuffer.
        {
            let mut blit_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("visualizer blit pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let (tex_w, tex_h) = pipeline.msaa_size;
            blit_pass.set_viewport(
                clip_bounds.x as f32,
                clip_bounds.y as f32,
                tex_w as f32,
                tex_h as f32,
                0.0,
                1.0,
            );
            blit_pass.set_scissor_rect(clip_bounds.x, clip_bounds.y, width, height);
            if self.crt_enabled {
                blit_pass.set_pipeline(&pipeline.crt_pipeline);
                blit_pass.set_bind_group(0, display_bg, &[]);
                blit_pass.set_bind_group(1, &pipeline.crt_uniform_bind_group, &[]);
            } else {
                blit_pass.set_pipeline(&pipeline.blit_pipeline);
                blit_pass.set_bind_group(0, display_bg, &[]);
            }
            blit_pass.draw(0..3, 0..1);
        }

        // Echo: redraw particles fresh on top of the warped ring.
        if self.echo_enabled && self.particle_count > 0 {
            let mut particle_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("visualizer echo fresh-particle pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let (tex_w, tex_h) = pipeline.msaa_size;
            particle_pass.set_viewport(
                clip_bounds.x as f32,
                clip_bounds.y as f32,
                tex_w as f32,
                tex_h as f32,
                0.0,
                1.0,
            );
            particle_pass.set_scissor_rect(clip_bounds.x, clip_bounds.y, width, height);
            Self::draw_particles(
                &pipeline.particle_pipeline,
                &pipeline.bind_group,
                self.particle_count,
                &mut particle_pass,
            );
        }

        // Pass 3: Bloom composite.
        if let Some((.., bg_bloom)) = bloom_views {
            let mut bloom_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("visualizer bloom composite pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            let (tex_w, tex_h) = pipeline.msaa_size;
            bloom_pass.set_viewport(
                clip_bounds.x as f32,
                clip_bounds.y as f32,
                tex_w as f32,
                tex_h as f32,
                0.0,
                1.0,
            );
            bloom_pass.set_scissor_rect(clip_bounds.x, clip_bounds.y, width, height);
            bloom_pass.set_pipeline(&pipeline.bloom_composite_pipeline);
            bloom_pass.set_bind_group(0, bg_bloom, &[]);
            bloom_pass.draw(0..3, 0..1);
        }
    }
}

// --- Widget program ---

#[derive(Clone)]
pub struct ShaderVisualizer {
    pub(super) backend: std::sync::Arc<crate::visualizer::VisualizerState>,
    pub(super) mode: VizMode,
    pub(super) config: VizConfig,
}

impl ShaderVisualizer {
    pub fn new(
        backend: std::sync::Arc<crate::visualizer::VisualizerState>,
        mode: VizMode,
        config: VizConfig,
    ) -> Self {
        Self { backend, mode, config }
    }
}

impl<Message: Clone + Send + 'static> shader::Program<Message> for ShaderVisualizer {
    // Option<VizState> satisfies Default (= None); lazily initialized in update().
    type State = Option<VizState>;
    type Primitive = VisualizerPrimitive;

    fn update(
        &self,
        state: &mut Option<VizState>,
        _event: &Event,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Option<shader::Action<Message>> {
        let s = state.get_or_insert_with(|| {
            VizState::new(self.backend.clone(), self.config.scope_particle_count as usize)
        });

        // Tick peak/beat/particle state each time an event arrives.
        // The 16ms VisualizerTick subscription in app.rs drives repaint cadence,
        // which in turn causes this update() to be called via RedrawRequested.
        s.tick(
            self.config.peak_hold_time,
            self.config.peak_fade_time,
            self.config.scope_radius,
            self.config.scope_sensitivity,
            self.config.scope_particles,
            self.config.scope_particle_speed,
        );

        let (mode_trails, mode_echo) = match self.mode {
            VizMode::Bars => (self.config.bars_trails, self.config.bars_echo),
            VizMode::Lines => (self.config.lines_trails, self.config.lines_echo),
            VizMode::Scope => (self.config.scope_trails, self.config.scope_echo),
        };
        let feedback_draining = (mode_trails > 0.001 || mode_echo > 0.001) && s.trail_draining();

        if s.is_dirty() || feedback_draining {
            Some(shader::Action::request_redraw())
        } else {
            None
        }
    }

    fn draw(
        &self,
        state: &Option<VizState>,
        _cursor: mouse::Cursor,
        bounds: Rectangle,
    ) -> Self::Primitive {
        let mut cfg = self.config.clone();
        // Auto-compute bar sizing from canvas width when not set.
        if cfg.bar_width <= 0.001 {
            let bar_count = cfg.bars_max_bars.max(1) as f32;
            let slot = bounds.width / bar_count;
            cfg.bar_spacing = (slot * 0.12_f32).max(0.5);
            cfg.bar_width = (slot - cfg.bar_spacing).max(1.0);
            cfg.edge_spacing = cfg.bar_spacing * 0.5;
        }
        match state {
            Some(s) => VisualizerPrimitive::new(s, self.mode, &cfg),
            None => VisualizerPrimitive::new(
                &VizState::new(self.backend.clone(), cfg.scope_particle_count as usize),
                self.mode,
                &cfg,
            ),
        }
    }
}
