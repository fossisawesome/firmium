//! GPU visualizer renderer (wgpu).
//!
//! Renders the orb / bars / oscilloscope modes to an offscreen texture in a
//! dedicated thread, reads the RGBA pixels back, and streams each frame to the
//! frontend over a Tauri `ipc::Channel` as raw bytes. The Svelte side only does
//! `putImageData` — no WebGL, no shaders, no animation loop.
//!
//! Analysis data (bass / bars / wave) is read in-process from the shared
//! `VisualizerState`; the old `firmium:audio-analysis` JS event is gone.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tauri::ipc::{Channel, InvokeResponseBody};

use crate::visualizer::VisualizerState;

/// Upper bound on the render target so an oversized panel can't make readback
/// (GPU copy + IPC) pathologically expensive.
const MAX_DIM: u32 = 1600;
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Orb,
    Bars,
    Oscilloscope,
}

impl Mode {
    fn parse(s: &str) -> Mode {
        match s {
            "bars" => Mode::Bars,
            "oscilloscope" => Mode::Oscilloscope,
            _ => Mode::Orb,
        }
    }
}

/// Send + Sync control surface poked by the Tauri commands and read by the
/// render thread. Holds no wgpu objects, so it's cheap to share via `Arc`.
pub struct GpuControl {
    rendering: AtomicBool,
    started: AtomicBool,
    mode: Mutex<Mode>,
    palette: Mutex<[[f32; 3]; 3]>,
    size: Mutex<(u32, u32)>,
    channel: Mutex<Option<Channel<InvokeResponseBody>>>,
}

impl GpuControl {
    pub fn new() -> Self {
        GpuControl {
            rendering: AtomicBool::new(false),
            started: AtomicBool::new(false),
            mode: Mutex::new(Mode::Orb),
            palette: Mutex::new([
                [124.0 / 255.0, 92.0 / 255.0, 1.0],
                [170.0 / 255.0, 136.0 / 255.0, 1.0],
                [85.0 / 255.0, 51.0 / 255.0, 204.0 / 255.0],
            ]),
            size: Mutex::new((0, 0)),
            channel: Mutex::new(None),
        }
    }

    pub fn set_mode(&self, mode: &str) {
        *self.mode.lock() = Mode::parse(mode);
    }

    pub fn set_palette(&self, palette: [[f32; 3]; 3]) {
        *self.palette.lock() = palette;
    }

    pub fn set_size(&self, w: u32, h: u32) {
        *self.size.lock() = (w.clamp(1, MAX_DIM), h.clamp(1, MAX_DIM));
    }

    pub fn stop(&self) {
        self.rendering.store(false, Ordering::Relaxed);
        *self.channel.lock() = None;
    }
}

/// Begin (or resume) streaming frames over `channel` at the given size. Spawns
/// the render thread on first call; later calls just swap channel/size and flip
/// the rendering flag back on.
pub fn start(
    control: Arc<GpuControl>,
    state: Arc<VisualizerState>,
    channel: Channel<InvokeResponseBody>,
    width: u32,
    height: u32,
) {
    control.set_size(width, height);
    *control.channel.lock() = Some(channel);
    control.rendering.store(true, Ordering::Relaxed);

    if !control.started.swap(true, Ordering::SeqCst) {
        std::thread::Builder::new()
            .name("visualizer-gpu".into())
            .spawn(move || render_loop(control, state))
            .ok();
    }
}

fn render_loop(control: Arc<GpuControl>, state: Arc<VisualizerState>) {
    let mut renderer = match pollster::block_on(Renderer::new()) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[visualizer-gpu] init failed: {e}");
            control.started.store(false, Ordering::SeqCst);
            return;
        }
    };

    loop {
        let frame_start = Instant::now();

        if !control.rendering.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(50));
            continue;
        }

        let (w, h) = *control.size.lock();
        let channel = control.channel.lock().clone();
        let Some(channel) = channel else {
            std::thread::sleep(Duration::from_millis(50));
            continue;
        };

        renderer.resize(w, h);
        let mode = *control.mode.lock();
        let palette = *control.palette.lock();
        let (bass, bars, wave) = state.snapshot();

        let pixels = renderer.render(mode, palette, bass, &bars, &wave);
        let _ = channel.send(InvokeResponseBody::Raw(pixels));

        if let Some(rem) = FRAME_INTERVAL.checked_sub(frame_start.elapsed()) {
            std::thread::sleep(rem);
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    res: [f32; 2],
    _pad0: [f32; 2],
    bass: f32,
    clock: f32,
    breathe: f32,
    _pad1: f32,
    pal: [[f32; 4]; 3],
    bars: [[f32; 4]; 8],
    wave: [[f32; 4]; 32],
}

struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    uniform_buf: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    orb: wgpu::RenderPipeline,
    bars: wgpu::RenderPipeline,
    scope: wgpu::RenderPipeline,
    target: Option<wgpu::Texture>,
    view: Option<wgpu::TextureView>,
    readback: Option<wgpu::Buffer>,
    size: (u32, u32),
    padded_bpr: u32,
    start: Instant,
    smooth_bass: f32,
}

const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

impl Renderer {
    async fn new() -> Result<Renderer, String> {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .map_err(|e| format!("no GPU adapter: {e}"))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("visualizer-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|e| format!("request_device: {e}"))?;

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visualizer-uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("visualizer-bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("visualizer-bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("visualizer-pl"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("visualizer-wgsl"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });

        let make = |vs: &str, fs: &str, blend: wgpu::BlendState, topology: wgpu::PrimitiveTopology| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("visualizer-pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vs),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: TARGET_FORMAT,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };

        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };

        let orb = make("vs_orb", "fs_orb", additive, wgpu::PrimitiveTopology::TriangleList);
        let bars = make("vs_bars", "fs_bars", wgpu::BlendState::ALPHA_BLENDING, wgpu::PrimitiveTopology::TriangleList);
        let scope = make("vs_scope", "fs_scope", wgpu::BlendState::ALPHA_BLENDING, wgpu::PrimitiveTopology::LineStrip);

        Ok(Renderer {
            device,
            queue,
            uniform_buf,
            bind_group,
            orb,
            bars,
            scope,
            target: None,
            view: None,
            readback: None,
            size: (0, 0),
            padded_bpr: 0,
            start: Instant::now(),
            smooth_bass: 0.0,
        })
    }

    fn resize(&mut self, w: u32, h: u32) {
        if self.size == (w, h) && self.target.is_some() {
            return;
        }
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let unpadded_bpr = w * 4;
        let padded_bpr = unpadded_bpr.div_ceil(align) * align;

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("visualizer-target"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TARGET_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visualizer-readback"),
            size: (padded_bpr * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        self.target = Some(texture);
        self.view = Some(view);
        self.readback = Some(readback);
        self.size = (w, h);
        self.padded_bpr = padded_bpr;
    }

    fn render(
        &mut self,
        mode: Mode,
        palette: [[f32; 3]; 3],
        bass: f32,
        bars: &[f32],
        wave: &[f32],
    ) -> Vec<u8> {
        let (w, h) = self.size;
        self.smooth_bass += (bass - self.smooth_bass) * 0.25;
        let t = self.start.elapsed().as_secs_f32();

        let mut pal = [[0.0f32; 4]; 3];
        for (i, c) in palette.iter().enumerate() {
            pal[i] = [c[0], c[1], c[2], 1.0];
        }
        let mut bars4 = [[0.0f32; 4]; 8];
        for (i, &b) in bars.iter().enumerate().take(32) {
            bars4[i / 4][i % 4] = b;
        }
        let mut wave4 = [[0.0f32; 4]; 32];
        for (i, &v) in wave.iter().enumerate().take(128) {
            wave4[i / 4][i % 4] = v;
        }

        let uniforms = Uniforms {
            res: [w as f32, h as f32],
            _pad0: [0.0; 2],
            bass: self.smooth_bass,
            clock: (t / 8.0).fract(),
            breathe: (t / 2.4).fract(),
            _pad1: 0.0,
            pal,
            bars: bars4,
            wave: wave4,
        };
        self.queue.write_buffer(&self.uniform_buf, 0, bytemuck::bytes_of(&uniforms));

        let view = self.view.as_ref().unwrap();
        let target = self.target.as_ref().unwrap();
        let readback = self.readback.as_ref().unwrap();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("visualizer-enc") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("visualizer-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.bind_group, &[]);
            match mode {
                Mode::Orb => {
                    pass.set_pipeline(&self.orb);
                    pass.draw(0..3, 0..1);
                }
                Mode::Bars => {
                    pass.set_pipeline(&self.bars);
                    pass.draw(0..(32 * 6), 0..1);
                }
                Mode::Oscilloscope => {
                    pass.set_pipeline(&self.scope);
                    // 129 verts: vertex 128 closes the loop back to wave[0].
                    pass.draw(0..129, 0..1);
                }
            }
        }

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_bpr),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = readback.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let _ = rx.recv();

        let padded = self.padded_bpr as usize;
        let row = (w * 4) as usize;
        let mut out = vec![0u8; row * h as usize];
        {
            let mapped = slice.get_mapped_range();
            for y in 0..h as usize {
                let src = y * padded;
                let dst = y * row;
                out[dst..dst + row].copy_from_slice(&mapped[src..src + row]);
            }
        }
        readback.unmap();
        out
    }
}

const SHADER: &str = r#"
struct U {
  res: vec2<f32>,
  _pad0: vec2<f32>,
  bass: f32,
  clock: f32,
  breathe: f32,
  _pad1: f32,
  pal: array<vec4<f32>, 3>,
  bars: array<vec4<f32>, 8>,
  wave: array<vec4<f32>, 32>,
};
@group(0) @binding(0) var<uniform> u: U;

const TAU: f32 = 6.28318530718;

fn pal(t_in: f32) -> vec3<f32> {
  let t = fract(t_in);
  if (t < 0.333) { return mix(u.pal[0].xyz, u.pal[1].xyz, t / 0.333); }
  if (t < 0.666) { return mix(u.pal[1].xyz, u.pal[2].xyz, (t - 0.333) / 0.333); }
  return mix(u.pal[2].xyz, u.pal[0].xyz, (t - 0.666) / 0.334);
}

fn bar_at(i: i32) -> f32 { return u.bars[i / 4][i % 4]; }
fn wave_at(i: i32) -> f32 { return u.wave[i / 4][i % 4]; }

// ── Orb: fullscreen triangle + analytic fragment ──────────────────────────
@vertex
fn vs_orb(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
  let p = vec2<f32>(f32(vid % 2u), f32(vid / 2u));
  return vec4<f32>(p * 4.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_orb(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
  // wgpu framebuffer is top-origin; flip Y so the math matches the GL original.
  let fc = vec2<f32>(pos.x, u.res.y - pos.y);
  var p = fc / u.res - 0.5;
  p.x = p.x * (u.res.x / u.res.y);

  let d = length(p);
  let bf = sin(u.breathe * TAU) * 0.5 + 0.5;
  let orbR = 0.075 + bf * 0.022 + u.bass * 0.04;

  var col = vec3<f32>(0.0);

  for (var i = 0; i < 4; i = i + 1) {
    let f = f32(i) / 3.0;
    let r = orbR * (1.8 - f * 0.8);
    let a = 0.12 + f * 0.25;
    col = col + pal(u.clock) * a * exp(-d * d / (r * r) * 2.0);
  }

  let cm = exp(-d * d / (orbR * orbR) * 3.0);
  let ch = mix(vec3<f32>(1.0), pal(u.clock), smoothstep(0.0, orbR * 0.5, d));
  col = col + ch * cm * 0.9;

  for (var i = 0; i < 3; i = i + 1) {
    let ph = fract(u.clock + f32(i) / 3.0);
    let rr = orbR * (1.1 + ph * 2.2);
    let ra = (1.0 - ph) * (0.4 + u.bass * 0.4);
    var rc = pal(u.clock + 0.55);
    if (i % 2 == 0) { rc = pal(u.clock + 0.33); }
    let rw = max(0.5, 3.0 - ph * 2.5) / min(u.res.x, u.res.y);
    col = col + rc * ra * smoothstep(rw, 0.0, abs(d - rr));
  }

  for (var w = 0; w < 4; w = w + 1) {
    let ang = u.clock * TAU + f32(w) * (TAU / 4.0);
    let oR = orbR * (1.35 + sin(u.breathe * TAU + f32(w)) * 0.15);
    let wpos = vec2<f32>(cos(ang), sin(ang)) * oR;
    let wd = length(p - wpos);
    let wR = orbR * (0.18 + u.bass * 0.12);
    var wc = pal(u.clock + 0.50);
    if (w % 2 == 0) { wc = pal(u.clock + 0.17); }
    col = col + wc * 0.7 * exp(-wd * wd / (wR * wR) * 4.0);
  }

  let pa = pal(u.clock + 0.10);
  let pb = pal(u.clock + 0.40);
  let pc = pal(u.clock + 0.70);
  for (var k = 0; k < 28; k = k + 1) {
    let baseA = f32(k) / 28.0 * TAU;
    let speed = 0.3 + f32(k % 7) * 0.1;
    let age = fract(u.clock + f32(k) / 28.0);
    let ang = baseA + u.clock * 0.8 * TAU;
    let pd = orbR * (0.9 + age * 1.8 * (0.6 + u.bass * 0.8) * speed);
    let pp = vec2<f32>(cos(ang), sin(ang)) * pd;
    let dd = length(p - pp);
    let pr = max(0.004, (3.0 - age * 2.5) / min(u.res.x, u.res.y));
    var pkc = pc;
    if (age < 0.33) { pkc = pa; } else if (age < 0.66) { pkc = pb; }
    col = col + pkc * max(0.0, (1.0 - age) * 0.7) * exp(-dd * dd / (pr * pr));
  }

  return vec4<f32>(col, 1.0);
}

// ── Bars: 32 quads (6 verts each) from vertex_index ───────────────────────
struct BarsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) v_t: f32,
  @location(1) v_idx: f32,
};

@vertex
fn vs_bars(@builtin(vertex_index) vid: u32) -> BarsOut {
  let bar = i32(vid) / 6;
  let corner = i32(vid) % 6;

  let gap = 3.0;
  let barW = (u.res.x - gap * 31.0) / 32.0;
  let barH = bar_at(bar) * u.res.y;

  let xL = f32(bar) * (barW + gap);
  let xR = xL + barW;

  let right = (corner == 1 || corner == 2 || corner == 4);
  let top = (corner == 2 || corner == 4 || corner == 5);

  var px = xL;
  if (right) { px = xR; }
  var py = 0.0;
  if (top) { py = barH; }

  var out: BarsOut;
  out.v_t = select(0.0, py / barH, barH > 0.0);
  out.v_idx = f32(bar);
  out.pos = vec4<f32>((px / u.res.x) * 2.0 - 1.0, (py / u.res.y) * 2.0 - 1.0, 0.0, 1.0);
  return out;
}

@fragment
fn fs_bars(in: BarsOut) -> @location(0) vec4<f32> {
  let col = pal(u.clock + in.v_idx / 32.0);
  let bright = 0.45 + in.v_t * 0.55 + smoothstep(0.85, 1.0, in.v_t) * 0.5;
  let alpha = clamp(bright, 0.0, 1.0);
  return vec4<f32>(col * bright, alpha);
}

// ── Oscilloscope: 128-point ring as a LineStrip (129 verts to close) ──────
struct ScopeOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) v_i: f32,
};

@vertex
fn vs_scope(@builtin(vertex_index) vid: u32) -> ScopeOut {
  let i = i32(vid) % 128;
  let a = f32(i) / 128.0 * TAU;
  let r = 0.34 + wave_at(i) * 0.20;
  var p = vec2<f32>(cos(a), sin(a)) * r;
  if (u.res.x > u.res.y) { p.x = p.x * (u.res.y / u.res.x); } else { p.y = p.y * (u.res.x / u.res.y); }
  var out: ScopeOut;
  out.v_i = f32(i) / 128.0;
  out.pos = vec4<f32>(p, 0.0, 1.0);
  return out;
}

@fragment
fn fs_scope(in: ScopeOut) -> @location(0) vec4<f32> {
  return vec4<f32>(pal(u.clock + in.v_i), 1.0);
}
"#;
