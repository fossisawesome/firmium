//! GPU pipeline initialization for the visualizer.
//!
//! Ported from nokkvi with wgpu 27.0.1 API fixes:
//!   - `bind_group_layouts: &[Some(&x)]` → `&[&x]`
//!   - `immediate_size: 0` → `push_constant_ranges: &[]`
//!   - `multiview_mask: None` in RenderPipelineDescriptor → `multiview: None`
//!   - `multiview_mask: None` in RenderPassDescriptor → removed (field absent)

use iced::wgpu;

use super::shader::{BloomParams, CrtParams, EchoParams, TRAIL_FORMAT, Uniforms};

pub struct VisualizerPipeline {
    pub(super) bars_pipeline: wgpu::RenderPipeline,
    pub(super) bars_pipeline_msaa: wgpu::RenderPipeline,
    pub(super) lines_pipeline: wgpu::RenderPipeline,
    pub(super) lines_pipeline_msaa: wgpu::RenderPipeline,
    pub(super) scope_pipeline: wgpu::RenderPipeline,
    pub(super) scope_pipeline_msaa: wgpu::RenderPipeline,
    pub(super) particle_pipeline: wgpu::RenderPipeline,
    pub(super) particle_pipeline_msaa: wgpu::RenderPipeline,
    pub(super) scope_pipeline_beam: wgpu::RenderPipeline,
    pub(super) scope_pipeline_beam_msaa: wgpu::RenderPipeline,
    pub(super) uniform_buffer: wgpu::Buffer,
    pub(super) bar_buffer: wgpu::Buffer,
    pub(super) particle_buffer: wgpu::Buffer,
    pub(super) peak_buffer: wgpu::Buffer,
    pub(super) peak_alpha_buffer: wgpu::Buffer,
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) max_bars: usize,
    pub(super) msaa_texture: Option<(wgpu::Texture, wgpu::TextureView)>,
    pub(super) resolve_texture: Option<(wgpu::Texture, wgpu::TextureView)>,
    pub(super) ring_only_texture: Option<(wgpu::Texture, wgpu::TextureView)>,
    pub(super) blit_bind_group: Option<wgpu::BindGroup>,
    pub(super) blit_pipeline: wgpu::RenderPipeline,
    pub(super) blit_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) sampler: wgpu::Sampler,
    pub(super) msaa_size: (u32, u32),
    pub(super) format: wgpu::TextureFormat,
    pub(super) bloom_bright_pipeline: wgpu::RenderPipeline,
    pub(super) bloom_blur_v_pipeline: wgpu::RenderPipeline,
    pub(super) bloom_blur_h_pipeline: wgpu::RenderPipeline,
    pub(super) bloom_composite_pipeline: wgpu::RenderPipeline,
    pub(super) bloom_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) bloom_uniform_buffer: wgpu::Buffer,
    pub(super) bloom_texture_a: Option<(wgpu::Texture, wgpu::TextureView)>,
    pub(super) bloom_texture_b: Option<(wgpu::Texture, wgpu::TextureView)>,
    pub(super) bloom_bg_scene: Option<wgpu::BindGroup>,
    pub(super) bloom_bg_a: Option<wgpu::BindGroup>,
    pub(super) bloom_bg_b: Option<wgpu::BindGroup>,
    pub(super) trail_fade_pipeline: wgpu::RenderPipeline,
    pub(super) trail_max_pipeline: wgpu::RenderPipeline,
    pub(super) trail_texture: Option<(wgpu::Texture, wgpu::TextureView)>,
    pub(super) blit_bg_trail: Option<wgpu::BindGroup>,
    pub(super) trails_were_active: bool,
    pub(super) trail_needs_clear: bool,
    pub(super) echo_feedback_pipeline: wgpu::RenderPipeline,
    pub(super) echo_bind_group_layout: wgpu::BindGroupLayout,
    pub(super) echo_uniform_buffer: wgpu::Buffer,
    pub(super) echo_texture: Option<(wgpu::Texture, wgpu::TextureView)>,
    pub(super) echo_temp: Option<(wgpu::Texture, wgpu::TextureView)>,
    pub(super) echo_feedback_bg: Option<wgpu::BindGroup>,
    pub(super) blit_bg_echo: Option<wgpu::BindGroup>,
    pub(super) echo_were_active: bool,
    pub(super) echo_needs_clear: bool,
    pub(super) crt_pipeline: wgpu::RenderPipeline,
    pub(super) crt_uniform_buffer: wgpu::Buffer,
    pub(super) crt_uniform_bind_group: wgpu::BindGroup,
}

#[allow(clippy::too_many_arguments)]
fn build_visualizer_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    topology: wgpu::PrimitiveTopology,
    msaa: bool,
    label: &'static str,
    format: wgpu::TextureFormat,
    blend: wgpu::BlendState,
) -> wgpu::RenderPipeline {
    let multisample = if msaa {
        wgpu::MultisampleState { count: 4, mask: !0, alpha_to_coverage_enabled: false }
    } else {
        wgpu::MultisampleState::default()
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState { topology, ..Default::default() },
        depth_stencil: None,
        multisample,
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(blend),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview: None,
        cache: None,
    })
}

fn build_postprocess_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    entries: (&'static str, &'static str),
    blend: wgpu::BlendState,
    format: wgpu::TextureFormat,
    label: &'static str,
) -> wgpu::RenderPipeline {
    let (vs_entry, fs_entry) = entries;
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some(vs_entry),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fs_entry),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(blend),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview: None,
        cache: None,
    })
}

impl VisualizerPipeline {
    pub(crate) const MAX_BARS: usize = 2048;
    pub(crate) const MAX_PARTICLES: usize = 2048;

    pub(crate) fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visualizer uniform buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bar_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visualizer bar buffer"),
            size: (Self::MAX_BARS * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let peak_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visualizer peak buffer"),
            size: (Self::MAX_BARS * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let peak_alpha_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visualizer peak alpha buffer"),
            size: (Self::MAX_BARS * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let particle_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visualizer particle buffer"),
            size: (Self::MAX_PARTICLES * 8 * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("visualizer bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("visualizer bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: bar_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: peak_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: peak_alpha_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: particle_buffer.as_entire_binding(),
                },
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("visualizer pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let bars_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("visualizer bars shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shaders/bars.wgsl"
            ))),
        });

        let lines_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("visualizer lines shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shaders/lines.wgsl"
            ))),
        });

        let scope_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("visualizer scope shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shaders/scope.wgsl"
            ))),
        });

        let particle_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("visualizer particle shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shaders/particles.wgsl"
            ))),
        });

        let bars_pipeline = build_visualizer_pipeline(
            device, &layout, &bars_shader,
            wgpu::PrimitiveTopology::TriangleList, false,
            "visualizer bars pipeline", format, wgpu::BlendState::ALPHA_BLENDING,
        );
        let bars_pipeline_msaa = build_visualizer_pipeline(
            device, &layout, &bars_shader,
            wgpu::PrimitiveTopology::TriangleList, true,
            "visualizer bars pipeline (MSAA 4x)", format, wgpu::BlendState::ALPHA_BLENDING,
        );
        let lines_pipeline = build_visualizer_pipeline(
            device, &layout, &lines_shader,
            wgpu::PrimitiveTopology::TriangleList, false,
            "visualizer lines pipeline", format, wgpu::BlendState::ALPHA_BLENDING,
        );
        let lines_pipeline_msaa = build_visualizer_pipeline(
            device, &layout, &lines_shader,
            wgpu::PrimitiveTopology::TriangleList, true,
            "visualizer lines pipeline (MSAA 4x)", format, wgpu::BlendState::ALPHA_BLENDING,
        );
        let scope_pipeline = build_visualizer_pipeline(
            device, &layout, &scope_shader,
            wgpu::PrimitiveTopology::TriangleList, false,
            "visualizer scope pipeline", format, wgpu::BlendState::ALPHA_BLENDING,
        );
        let scope_pipeline_msaa = build_visualizer_pipeline(
            device, &layout, &scope_shader,
            wgpu::PrimitiveTopology::TriangleList, true,
            "visualizer scope pipeline (MSAA 4x)", format, wgpu::BlendState::ALPHA_BLENDING,
        );

        let additive_blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        };

        let particle_pipeline = build_visualizer_pipeline(
            device, &layout, &particle_shader,
            wgpu::PrimitiveTopology::TriangleList, false,
            "visualizer particle pipeline", format, additive_blend,
        );
        let particle_pipeline_msaa = build_visualizer_pipeline(
            device, &layout, &particle_shader,
            wgpu::PrimitiveTopology::TriangleList, true,
            "visualizer particle pipeline (MSAA 4x)", format, additive_blend,
        );
        let scope_pipeline_beam = build_visualizer_pipeline(
            device, &layout, &scope_shader,
            wgpu::PrimitiveTopology::TriangleList, false,
            "visualizer scope beam pipeline", format, additive_blend,
        );
        let scope_pipeline_beam_msaa = build_visualizer_pipeline(
            device, &layout, &scope_shader,
            wgpu::PrimitiveTopology::TriangleList, true,
            "visualizer scope beam pipeline (MSAA 4x)", format, additive_blend,
        );

        // Blit shader (inline WGSL)
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("visualizer blit shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
                r#"
struct VertexOut {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
}
@vertex
fn vs_blit(@builtin(vertex_index) idx: u32) -> VertexOut {
    var positions = array<vec2f, 3>(
        vec2f(-1.0, -1.0), vec2f(3.0, -1.0), vec2f(-1.0, 3.0),
    );
    var out: VertexOut;
    out.position = vec4f(positions[idx], 0.0, 1.0);
    out.uv = positions[idx] * vec2f(0.5, -0.5) + 0.5;
    return out;
}
@group(0) @binding(0) var t_resolve: texture_2d<f32>;
@group(0) @binding(1) var s_resolve: sampler;
@fragment
fn fs_blit(in: VertexOut) -> @location(0) vec4f {
    return textureSample(t_resolve, s_resolve, in.uv);
}
@fragment
fn fs_fade(in: VertexOut) -> @location(0) vec4f {
    return vec4f(0.0);
}
"#,
            )),
        });

        let blit_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("visualizer blit bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float {
                                filterable: true,
                            },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(
                            wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                ],
            });

        let blit_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("visualizer blit pipeline layout"),
            bind_group_layouts: &[&blit_bind_group_layout],
            push_constant_ranges: &[],
        });

        let premultiplied_blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        };

        let blit_pipeline = build_postprocess_pipeline(
            device, &blit_layout, &blit_shader,
            ("vs_blit", "fs_blit"), premultiplied_blend, format,
            "visualizer blit pipeline",
        );

        let trail_fade_blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::Constant,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::Zero,
                dst_factor: wgpu::BlendFactor::Constant,
                operation: wgpu::BlendOperation::Add,
            },
        };
        let trail_max_blend = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Max,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Max,
            },
        };
        let trail_fade_pipeline = build_postprocess_pipeline(
            device, &blit_layout, &blit_shader,
            ("vs_blit", "fs_fade"), trail_fade_blend, TRAIL_FORMAT,
            "visualizer trail fade pipeline",
        );
        let trail_max_pipeline = build_postprocess_pipeline(
            device, &blit_layout, &blit_shader,
            ("vs_blit", "fs_blit"), trail_max_blend, TRAIL_FORMAT,
            "visualizer trail max pipeline",
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("visualizer blit sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Bloom
        let bloom_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("visualizer bloom shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shaders/bloom.wgsl"
            ))),
        });

        let bloom_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("visualizer bloom bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float {
                                filterable: true,
                            },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(
                            wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let bloom_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visualizer bloom uniform buffer"),
            size: std::mem::size_of::<BloomParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bloom_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("visualizer bloom pipeline layout"),
            bind_group_layouts: &[&bloom_bind_group_layout],
            push_constant_ranges: &[],
        });

        let additive_blend_one = wgpu::BlendState {
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

        let bloom_bright_pipeline = build_postprocess_pipeline(
            device, &bloom_layout, &bloom_shader,
            ("vs_main", "fs_bright_h"), wgpu::BlendState::REPLACE, format,
            "visualizer bloom bright/H pipeline",
        );
        let bloom_blur_v_pipeline = build_postprocess_pipeline(
            device, &bloom_layout, &bloom_shader,
            ("vs_main", "fs_blur_v"), wgpu::BlendState::REPLACE, format,
            "visualizer bloom blur V pipeline",
        );
        let bloom_blur_h_pipeline = build_postprocess_pipeline(
            device, &bloom_layout, &bloom_shader,
            ("vs_main", "fs_blur_h"), wgpu::BlendState::REPLACE, format,
            "visualizer bloom blur H pipeline",
        );
        let bloom_composite_pipeline = build_postprocess_pipeline(
            device, &bloom_layout, &bloom_shader,
            ("vs_main", "fs_composite"), additive_blend_one, format,
            "visualizer bloom composite pipeline",
        );

        // Echo
        let echo_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("visualizer echo shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shaders/echo.wgsl"
            ))),
        });
        let texture_entry = |binding: u32| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let echo_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("visualizer echo bind group layout"),
                entries: &[
                    texture_entry(0),
                    texture_entry(1),
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(
                            wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let echo_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visualizer echo uniform buffer"),
            size: std::mem::size_of::<EchoParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let echo_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("visualizer echo pipeline layout"),
            bind_group_layouts: &[&echo_bind_group_layout],
            push_constant_ranges: &[],
        });
        let echo_feedback_pipeline = build_postprocess_pipeline(
            device, &echo_layout, &echo_shader,
            ("vs_echo", "fs_echo"), wgpu::BlendState::REPLACE, TRAIL_FORMAT,
            "visualizer echo feedback pipeline",
        );

        // CRT
        let crt_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("visualizer crt shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shaders/crt.wgsl"
            ))),
        });
        let crt_uniform_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("visualizer crt uniform bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let crt_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("visualizer crt uniform buffer"),
            size: std::mem::size_of::<CrtParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let crt_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("visualizer crt uniform bind group"),
            layout: &crt_uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: crt_uniform_buffer.as_entire_binding(),
            }],
        });
        let crt_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("visualizer crt pipeline layout"),
            bind_group_layouts: &[&blit_bind_group_layout, &crt_uniform_layout],
            push_constant_ranges: &[],
        });
        let crt_pipeline = build_postprocess_pipeline(
            device, &crt_layout, &crt_shader,
            ("vs_crt", "fs_crt"), premultiplied_blend, format,
            "visualizer crt pipeline",
        );

        Self {
            bars_pipeline,
            bars_pipeline_msaa,
            lines_pipeline,
            lines_pipeline_msaa,
            scope_pipeline,
            scope_pipeline_msaa,
            particle_pipeline,
            particle_pipeline_msaa,
            scope_pipeline_beam,
            scope_pipeline_beam_msaa,
            uniform_buffer,
            bar_buffer,
            particle_buffer,
            peak_buffer,
            peak_alpha_buffer,
            bind_group,
            max_bars: Self::MAX_BARS,
            msaa_texture: None,
            resolve_texture: None,
            ring_only_texture: None,
            blit_bind_group: None,
            blit_pipeline,
            blit_bind_group_layout,
            sampler,
            msaa_size: (0, 0),
            format,
            bloom_bright_pipeline,
            bloom_blur_v_pipeline,
            bloom_blur_h_pipeline,
            bloom_composite_pipeline,
            bloom_bind_group_layout,
            bloom_uniform_buffer,
            bloom_texture_a: None,
            bloom_texture_b: None,
            bloom_bg_scene: None,
            bloom_bg_a: None,
            bloom_bg_b: None,
            trail_fade_pipeline,
            trail_max_pipeline,
            trail_texture: None,
            blit_bg_trail: None,
            trails_were_active: false,
            trail_needs_clear: false,
            echo_feedback_pipeline,
            echo_bind_group_layout,
            echo_uniform_buffer,
            echo_texture: None,
            echo_temp: None,
            echo_feedback_bg: None,
            blit_bg_echo: None,
            echo_were_active: false,
            echo_needs_clear: false,
            crt_pipeline,
            crt_uniform_buffer,
            crt_uniform_bind_group,
        }
    }
}

impl iced::widget::shader::Pipeline for VisualizerPipeline {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        Self::new(device, queue, format)
    }
}
