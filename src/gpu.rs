use std::time::Instant;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::fish::{DisturbanceVertex, SpriteVertex, Vertex};
use crate::skin::{DecodedSprite, Skin};
use crate::weather::WeatherParams;

pub struct FishFrame<'a> {
    pub outer: &'a [Vertex],
    pub body: &'a [Vertex],
    pub shadow: &'a [Vertex],
    pub disturbance: &'a [DisturbanceVertex],
}

pub struct SpriteFrame<'a> {
    pub koi: &'a [Vec<SpriteVertex>],
    pub leaves: &'a [Vec<SpriteVertex>],
}

pub struct SurfaceFrame<'a> {
    pub shadow: &'a [Vertex],
    pub leaves: &'a [Vertex],
    pub veins: &'a [Vertex],
    pub flowers: &'a [Vertex],
    pub duckweed_shadow: &'a [Vertex],
    pub duckweed_leaves: &'a [Vertex],
    pub duckweed_highlights: &'a [Vertex],
    pub tiny_shadow: &'a [Vertex],
    pub tiny_shapes: &'a [Vertex],
    pub tiny_details: &'a [Vertex],
    pub butterfly_shadow: &'a [Vertex],
    pub butterflies: &'a [Vertex],
}

pub const POND_WIDTH: u32 = 480;
pub const POND_HEIGHT: u32 = 270;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BedUniforms {
    resolution: [f32; 2],
    vertical_tone: f32,
    grain_scale: f32,
    edge_darkening: f32,
    pad0: f32,
    pad1: f32,
    pad2: f32,
    deep: [f32; 4],
    shallow: [f32; 4],
    speck: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct WeatherUniforms {
    time: f32,
    brightness: f32,
    contrast: f32,
    saturation: f32,
    vignette: f32,
    cloud_strength: f32,
    light_strength: f32,
    pad0: f32,
    light_direction: [f32; 2],
    pad1: [f32; 2],
    tint: [f32; 4],
    light_color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FishUniforms {
    resolution: [f32; 2],
    pad0: [f32; 2],
    shadow_color: [f32; 4],
    shadow_opacity: f32,
    pad1: f32,
    pad2: f32,
    pad3: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DisturbanceUniforms {
    resolution: [f32; 2],
    time: f32,
    strength: f32,
    wave_frequency: f32,
    wave_speed: f32,
    pad0: f32,
    pad1: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SpriteUniforms {
    resolution: [f32; 2],
    pad: [f32; 2],
}

struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl Target {
    fn new_sized(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        label: &str,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Self {
            view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
            texture,
        }
    }
}


fn make_targets(
    device: &wgpu::Device,
    scale: u32,
) -> (u32, u32, Target, Target, Target) {
    let width = POND_WIDTH * scale;
    let height = POND_HEIGHT * scale;
    let underwater = Target::new_sized(
        device,
        wgpu::TextureFormat::Rgba8Unorm,
        width,
        height,
        "underwater",
    );
    let composite = Target::new_sized(
        device,
        wgpu::TextureFormat::Rgba8Unorm,
        width,
        height,
        "composite",
    );
    let disturbance = Target::new_sized(
        device,
        wgpu::TextureFormat::Rg16Float,
        width.div_ceil(2),
        height.div_ceil(2),
        "disturbance",
    );
    (width, height, underwater, composite, disturbance)
}

fn make_samplers(device: &wgpu::Device, scale: u32) -> (wgpu::Sampler, wgpu::Sampler) {
    let filter = if scale > 1 {
        wgpu::FilterMode::Linear
    } else {
        wgpu::FilterMode::Nearest
    };
    let present = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("present"),
        mag_filter: filter,
        min_filter: filter,
        ..Default::default()
    });
    let linear = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("linear"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    (present, linear)
}

fn make_water_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    underwater: &Target,
    disturbance: &Target,
    sampler: &wgpu::Sampler,
    uniforms: &wgpu::Buffer,
    ripples: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("water bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&underwater.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&disturbance.view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: uniforms.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: ripples.as_entire_binding(),
            },
        ],
    })
}

fn make_weather_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    composite: &Target,
    sampler: &wgpu::Sampler,
    uniforms: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("weather bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&composite.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: uniforms.as_entire_binding(),
            },
        ],
    })
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_format: wgpu::TextureFormat,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    size: (u32, u32),

    scale: u32,
    render_width: u32,
    render_height: u32,
    underwater: Target,
    composite: Target,
    disturbance: Target,
    present_sampler: wgpu::Sampler,
    linear_sampler: wgpu::Sampler,
    _bed_pipeline: wgpu::RenderPipeline,
    _bed_uniforms: wgpu::Buffer,
    _bed_bind_group: wgpu::BindGroup,
    _water_pipeline: wgpu::RenderPipeline,
    _water_uniforms: wgpu::Buffer,
    _ripple_buffer: wgpu::Buffer,
    water_layout: wgpu::BindGroupLayout,
    water_bind_group: wgpu::BindGroup,
    fish_pipeline: wgpu::RenderPipeline,
    fish_shadow_pipeline: wgpu::RenderPipeline,
    fish_uniforms: wgpu::Buffer,
    fish_bind_group: wgpu::BindGroup,
    disturbance_pipeline: wgpu::RenderPipeline,
    disturbance_uniforms: wgpu::Buffer,
    disturbance_bind_group: wgpu::BindGroup,
    outer_buffer: wgpu::Buffer,
    body_buffer: wgpu::Buffer,
    shadow_buffer: wgpu::Buffer,
    disturbance_buffer: wgpu::Buffer,
    lotus_shadow_buffer: wgpu::Buffer,
    lotus_leaf_buffer: wgpu::Buffer,
    lotus_vein_buffer: wgpu::Buffer,
    lotus_flower_buffer: wgpu::Buffer,
    lotus_shadow_uniforms: wgpu::Buffer,
    lotus_shadow_bind_group: wgpu::BindGroup,
    line_pipeline: wgpu::RenderPipeline,
    sprite_pipeline: wgpu::RenderPipeline,
    sprite_texture_layout: wgpu::BindGroupLayout,
    _sprite_uniforms: wgpu::Buffer,
    sprite_uniform_group: wgpu::BindGroup,
    koi_sprite_buffers: Vec<wgpu::Buffer>,
    leaf_sprite_buffers: Vec<wgpu::Buffer>,
    koi_sprite_groups: Vec<Option<wgpu::BindGroup>>,
    leaf_sprite_groups: Vec<Option<wgpu::BindGroup>>,
    duckweed_shadow_buffer: wgpu::Buffer,
    duckweed_leaf_buffer: wgpu::Buffer,
    duckweed_highlight_buffer: wgpu::Buffer,
    duckweed_shadow_uniforms: wgpu::Buffer,
    duckweed_shadow_bind_group: wgpu::BindGroup,
    tiny_shadow_buffer: wgpu::Buffer,
    tiny_shape_buffer: wgpu::Buffer,
    tiny_detail_buffer: wgpu::Buffer,
    tiny_shadow_uniforms: wgpu::Buffer,
    tiny_shadow_bind_group: wgpu::BindGroup,
    butterfly_shadow_buffer: wgpu::Buffer,
    butterfly_buffer: wgpu::Buffer,
    butterfly_shadow_uniforms: wgpu::Buffer,
    butterfly_shadow_bind_group: wgpu::BindGroup,
    current_times: [f32; 3],
    last_time: f32,
    _weather_pipeline: wgpu::RenderPipeline,
    weather_pipeline_rgba: wgpu::RenderPipeline,
    _weather_uniforms: wgpu::Buffer,
    weather_layout: wgpu::BindGroupLayout,
    weather_bind_group: wgpu::BindGroup,

    start: Instant,
}

impl Renderer {
    pub async fn new(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        initial_scale: u32,
    ) -> Self {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .expect("no suitable GPU adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .expect("failed to request device");

        let caps = surface.get_capabilities(&adapter);
        let surface_format = [
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba8Unorm,
        ]
        .into_iter()
        .find(|f| caps.formats.contains(f))
        .unwrap_or_else(|| {
            caps.formats
                .iter()
                .copied()
                .find(|f| !f.is_srgb())
                .unwrap_or(caps.formats[0])
        });
        log::info!("surface format {surface_format:?}");

        let shader = |code: String, label: &str| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(code.into()),
            })
        };

        let fullscreen = include_str!("shaders/fullscreen.wgsl");
        let bed_module = shader(
            format!("{fullscreen}\n{}", include_str!("shaders/bed.wgsl")),
            "bed",
        );
        let weather_module = shader(
            format!("{fullscreen}\n{}", include_str!("shaders/weather.wgsl")),
            "weather",
        );
        let water_module = shader(
            format!("{fullscreen}\n{}", include_str!("shaders/water.wgsl")),
            "water",
        );
        let fish_module = shader(include_str!("shaders/fish.wgsl").to_string(), "fish");
        let disturbance_module =
            shader(include_str!("shaders/disturbance.wgsl").to_string(), "disturbance");

        let bed_uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bed uniforms"),
            contents: bytemuck::bytes_of(&BedUniforms {
                resolution: [POND_WIDTH as f32, POND_HEIGHT as f32],
                vertical_tone: 0.8,
                grain_scale: 0.54,
                edge_darkening: 0.57,
                pad0: 0.0,
                pad1: 0.0,
                pad2: 0.0,
                deep: [0.486, 0.718, 0.631, 1.0],
                shallow: [0.145, 0.395, 0.255, 1.0],
                speck: [0.02, 0.065, 0.04, 1.0],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let weather_uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("weather uniforms"),
            contents: bytemuck::bytes_of(&WeatherUniforms {
                time: 0.0,
                brightness: 1.0,
                contrast: 1.0,
                saturation: 1.0,
                vignette: 0.0,
                cloud_strength: 0.0,
                light_strength: 0.0,
                pad0: 0.0,
                light_direction: [-0.58, 0.82],
                pad1: [0.0, 0.0],
                tint: [1.0, 1.0, 1.0, 1.0],
                light_color: [1.0, 0.94, 0.72, 1.0],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let scale = initial_scale.clamp(1, 4);
        let (render_width, render_height, underwater, composite, disturbance) =
            make_targets(&device, scale);
        log::info!("render scale {scale}: {render_width}x{render_height}");
        let (sampler, linear_sampler) = make_samplers(&device, scale);

        let bed_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bed layout"),
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
        let bed_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bed bind group"),
            layout: &bed_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: bed_uniforms.as_entire_binding(),
            }],
        });

        let weather_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("weather layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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
        let weather_bind_group = make_weather_bind_group(
            &device,
            &weather_layout,
            &composite,
            &sampler,
            &weather_uniforms,
        );

        let fullscreen_pipeline = |module: &wgpu::ShaderModule,
                                   layout: &wgpu::BindGroupLayout,
                                   label: &str,
                                   format: wgpu::TextureFormat| {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[Some(layout)],
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };


        let fish_uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("fish uniforms"),
            contents: bytemuck::bytes_of(&FishUniforms {
                resolution: [POND_WIDTH as f32, POND_HEIGHT as f32],
                pad0: [0.0, 0.0],
                shadow_color: [11.0 / 255.0, 33.0 / 255.0, 30.0 / 255.0, 1.0],
                shadow_opacity: 1.0,
                pad1: 0.0,
                pad2: 0.0,
                pad3: 0.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let fish_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fish layout"),
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
        let fish_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fish bind group"),
            layout: &fish_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: fish_uniforms.as_entire_binding(),
            }],
        });

        let disturbance_uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("disturbance uniforms"),
            contents: bytemuck::bytes_of(&DisturbanceUniforms {
                resolution: [POND_WIDTH as f32, POND_HEIGHT as f32],
                time: 0.0,
                strength: 2.3,
                wave_frequency: 8.5,
                wave_speed: 2.4,
                pad0: 0.0,
                pad1: 0.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let disturbance_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("disturbance layout"),
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
        let disturbance_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("disturbance bind group"),
            layout: &disturbance_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: disturbance_uniforms.as_entire_binding(),
            }],
        });

        let vertex_buffer = |label: &str, size: u64| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let outer_buffer = vertex_buffer("koi outer", 24000 * 20);
        let body_buffer = vertex_buffer("koi body", 24000 * 20);
        let shadow_buffer = vertex_buffer("koi shadows", 24000 * 20);
        let disturbance_buffer = vertex_buffer(
            "koi disturbance",
            512 * std::mem::size_of::<DisturbanceVertex>() as u64,
        );
        let lotus_shadow_buffer = vertex_buffer("lotus shadows", 4000 * 20);
        let lotus_leaf_buffer = vertex_buffer("lotus leaves", 4000 * 20);
        let lotus_vein_buffer = vertex_buffer("lotus veins", 2000 * 20);
        let lotus_flower_buffer = vertex_buffer("lotus flowers", 2000 * 20);
        let lotus_shadow_uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lotus shadow uniforms"),
            contents: bytemuck::bytes_of(&FishUniforms {
                resolution: [POND_WIDTH as f32, POND_HEIGHT as f32],
                pad0: [0.0, 0.0],
                shadow_color: [
                    crate::lotus::SHADOW_COLOR[0],
                    crate::lotus::SHADOW_COLOR[1],
                    crate::lotus::SHADOW_COLOR[2],
                    1.0,
                ],
                shadow_opacity: crate::lotus::SHADOW_OPACITY,
                pad1: 0.0,
                pad2: 0.0,
                pad3: 0.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let duckweed_shadow_buffer = vertex_buffer("duckweed shadows", 50000 * 20);
        let duckweed_leaf_buffer = vertex_buffer("duckweed leaves", 50000 * 20);
        let duckweed_highlight_buffer = vertex_buffer("duckweed highlights", 15000 * 20);
        let duckweed_shadow_uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("duckweed shadow uniforms"),
            contents: bytemuck::bytes_of(&FishUniforms {
                resolution: [POND_WIDTH as f32, POND_HEIGHT as f32],
                pad0: [0.0, 0.0],
                shadow_color: [
                    crate::duckweed::SHADOW_COLOR[0],
                    crate::duckweed::SHADOW_COLOR[1],
                    crate::duckweed::SHADOW_COLOR[2],
                    1.0,
                ],
                shadow_opacity: crate::duckweed::SHADOW_OPACITY,
                pad1: 0.0,
                pad2: 0.0,
                pad3: 0.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let tiny_shadow_buffer = vertex_buffer("tiny shadows", 8000 * 20);
        let tiny_shape_buffer = vertex_buffer("tiny shapes", 16000 * 20);
        let tiny_detail_buffer = vertex_buffer("tiny details", 12000 * 20);
        let tiny_shadow_uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tiny shadow uniforms"),
            contents: bytemuck::bytes_of(&FishUniforms {
                resolution: [POND_WIDTH as f32, POND_HEIGHT as f32],
                pad0: [0.0, 0.0],
                shadow_color: [
                    crate::tiny::TINY_SHADOW_COLOR[0],
                    crate::tiny::TINY_SHADOW_COLOR[1],
                    crate::tiny::TINY_SHADOW_COLOR[2],
                    1.0,
                ],
                shadow_opacity: crate::tiny::SHADOW_OPACITY,
                pad1: 0.0,
                pad2: 0.0,
                pad3: 0.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let butterfly_shadow_buffer = vertex_buffer("butterfly shadows", 1000 * 20);
        let butterfly_buffer = vertex_buffer("butterflies", 2000 * 20);
        let butterfly_shadow_uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("butterfly shadow uniforms"),
            contents: bytemuck::bytes_of(&FishUniforms {
                resolution: [POND_WIDTH as f32, POND_HEIGHT as f32],
                pad0: [0.0, 0.0],
                shadow_color: [
                    crate::butterflies::SHADOW_COLOR[0],
                    crate::butterflies::SHADOW_COLOR[1],
                    crate::butterflies::SHADOW_COLOR[2],
                    1.0,
                ],
                shadow_opacity: crate::butterflies::SHADOW_OPACITY,
                pad1: 0.0,
                pad2: 0.0,
                pad3: 0.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let butterfly_shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("butterfly shadow bind group"),
            layout: &fish_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: butterfly_shadow_uniforms.as_entire_binding(),
            }],
        });
        let tiny_shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("tiny shadow bind group"),
            layout: &fish_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: tiny_shadow_uniforms.as_entire_binding(),
            }],
        });
        let duckweed_shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("duckweed shadow bind group"),
            layout: &fish_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: duckweed_shadow_uniforms.as_entire_binding(),
            }],
        });
        let lotus_shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lotus shadow bind group"),
            layout: &fish_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: lotus_shadow_uniforms.as_entire_binding(),
            }],
        });

        let fish_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("fish pipeline layout"),
                bind_group_layouts: &[Some(&fish_layout)],
                immediate_size: 0,
            });
        let fish_vertex_buffers = [Some(Vertex::LAYOUT)];
        let fish_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fish"),
            layout: Some(&fish_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &fish_module,
                entry_point: Some("vs_main"),
                buffers: &fish_vertex_buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fish_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let fish_shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fish shadows"),
            layout: Some(&fish_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &fish_module,
                entry_point: Some("vs_main"),
                buffers: &fish_vertex_buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fish_module,
                entry_point: Some("fs_shadow"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lotus veins"),
            layout: Some(&fish_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &fish_module,
                entry_point: Some("vs_main"),
                buffers: &fish_vertex_buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fish_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let sprite_module = shader(include_str!("shaders/sprite.wgsl").to_string(), "sprite");
        let sprite_texture_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sprite texture layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let sprite_uniform_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sprite uniform layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let sprite_uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sprite uniforms"),
            contents: bytemuck::bytes_of(&SpriteUniforms {
                resolution: [POND_WIDTH as f32, POND_HEIGHT as f32],
                pad: [0.0, 0.0],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let sprite_uniform_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite uniform group"),
            layout: &sprite_uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sprite_uniforms.as_entire_binding(),
            }],
        });
        let sprite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("sprite pipeline layout"),
                bind_group_layouts: &[Some(&sprite_texture_layout), Some(&sprite_uniform_layout)],
                immediate_size: 0,
            });
        let sprite_vertex_buffers = [Some(SpriteVertex::LAYOUT)];
        let sprite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite"),
            layout: Some(&sprite_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &sprite_module,
                entry_point: Some("vs_main"),
                buffers: &sprite_vertex_buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &sprite_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let mut koi_sprite_buffers = Vec::new();
        let mut leaf_sprite_buffers = Vec::new();
        for family in 0..6 {
            koi_sprite_buffers.push(vertex_buffer(
                &format!("koi sprite {family}"),
                256 * std::mem::size_of::<SpriteVertex>() as u64,
            ));
        }
        for palette in 0..2 {
            leaf_sprite_buffers.push(vertex_buffer(
                &format!("leaf sprite {palette}"),
                256 * std::mem::size_of::<SpriteVertex>() as u64,
            ));
        }

        let disturbance_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("disturbance pipeline layout"),
                bind_group_layouts: &[Some(&disturbance_layout)],
                immediate_size: 0,
            });
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
        let disturbance_vertex_buffers = [Some(DisturbanceVertex::LAYOUT)];
        let disturbance_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("disturbance"),
            layout: Some(&disturbance_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &disturbance_module,
                entry_point: Some("vs_main"),
                buffers: &disturbance_vertex_buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &disturbance_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rg16Float,
                    blend: Some(additive),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let water_values: [[f32; 4]; 24] = [
            [POND_WIDTH as f32, POND_HEIGHT as f32, 0.0, 0.0],
            [0.0, 1.0, 0.015, 0.0],
            [0.96, 1.02, 1.0, 1.0],
            [0.022, 0.068, 0.047, 1.0],
            [0.052, 0.155, 0.108, 1.0],
            [0.022, 0.068, 0.047, 1.0],
            [0.052, 0.155, 0.108, 1.0],
            [0.01, 0.034, 0.023, 1.0],
            [0.028, 0.09, 0.061, 1.0],
            [908.0, 10.0, 20.0, 0.0],
            [0.99, 0.15, 0.9, 0.0],
            [0.94, 0.34, 0.0, 0.0],
            [-0.38, 0.92, 0.0, 0.0],
            [0.71, 0.71, 0.0, 0.0],
            [22.0, 31.0, 59.0, 0.0],
            [0.92, 0.51, -6.38, 0.0],
            [1.2, 0.55, 0.28, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [2.2, 4.0, 62.0, 5.55],
            [1.25, 0.8, 30.0, 4.8],
            [0.9, 1.2, 17.0, 10.4],
            [0.3, 0.58, 0.18, 0.0],
            [0.62, 0.34, 0.18, 0.0],
            [0.7, 0.25, 0.45, 0.0],
        ];
        let water_uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("water uniforms"),
            contents: bytemuck::cast_slice(&water_values),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let ripple_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ripples"),
            contents: bytemuck::cast_slice(&[[0.0f32; 4]; 64]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let water_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("water layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
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
        let water_bind_group = make_water_bind_group(
            &device,
            &water_layout,
            &underwater,
            &disturbance,
            &linear_sampler,
            &water_uniforms,
            &ripple_buffer,
        );
        let water_pipeline = fullscreen_pipeline(
            &water_module,
            &water_layout,
            "water",
            wgpu::TextureFormat::Rgba8Unorm,
        );

        let bed_pipeline = fullscreen_pipeline(
            &bed_module,
            &bed_layout,
            "bed",
            wgpu::TextureFormat::Rgba8Unorm,
        );
        let weather_pipeline =
            fullscreen_pipeline(&weather_module, &weather_layout, "weather", surface_format);
        let weather_pipeline_rgba = fullscreen_pipeline(
            &weather_module,
            &weather_layout,
            "weather rgba",
            wgpu::TextureFormat::Rgba8Unorm,
        );

        Self {
            surface,
            device,
            queue,
            surface_format,
            surface_config: None,
            size: (0, 0),
            scale,
            render_width,
            render_height,
            underwater,
            composite,
            disturbance,
            present_sampler: sampler,
            linear_sampler,
            _bed_pipeline: bed_pipeline,
            _bed_uniforms: bed_uniforms,
            _bed_bind_group: bed_bind_group,
            _water_pipeline: water_pipeline,
            _water_uniforms: water_uniforms,
            _ripple_buffer: ripple_buffer,
            water_layout,
            water_bind_group,
            fish_pipeline,
            fish_shadow_pipeline,
            fish_uniforms,
            fish_bind_group,
            disturbance_pipeline,
            disturbance_uniforms,
            disturbance_bind_group,
            outer_buffer,
            body_buffer,
            shadow_buffer,
            disturbance_buffer,
            lotus_shadow_buffer,
            lotus_leaf_buffer,
            lotus_vein_buffer,
            lotus_flower_buffer,
            lotus_shadow_uniforms,
            lotus_shadow_bind_group,
            line_pipeline,
            sprite_pipeline,
            sprite_texture_layout,
            _sprite_uniforms: sprite_uniforms,
            sprite_uniform_group,
            koi_sprite_buffers,
            leaf_sprite_buffers,
            koi_sprite_groups: (0..6).map(|_| None).collect(),
            leaf_sprite_groups: (0..2).map(|_| None).collect(),
            duckweed_shadow_buffer,
            duckweed_leaf_buffer,
            duckweed_highlight_buffer,
            duckweed_shadow_uniforms,
            duckweed_shadow_bind_group,
            tiny_shadow_buffer,
            tiny_shape_buffer,
            tiny_detail_buffer,
            tiny_shadow_uniforms,
            tiny_shadow_bind_group,
            butterfly_shadow_buffer,
            butterfly_buffer,
            butterfly_shadow_uniforms,
            butterfly_shadow_bind_group,
            current_times: [0.0; 3],
            last_time: 0.0,
            _weather_pipeline: weather_pipeline,
            weather_pipeline_rgba,
            _weather_uniforms: weather_uniforms,
            weather_layout,
            weather_bind_group,
            start: Instant::now(),
        }
    }

    pub fn configure(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.size = (width, height);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            view_formats: vec![],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width,
            height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        self.surface.configure(&self.device, &config);
        self.surface_config = Some(config);
    }

    pub fn scale(&self) -> u32 {
        self.scale
    }

    pub fn set_scale(&mut self, scale: u32) -> Result<(), String> {
        if !(1..=4).contains(&scale) {
            return Err("scale must be between 1 and 4".to_string());
        }
        if scale == self.scale {
            return Ok(());
        }
        let (width, height, underwater, composite, disturbance) =
            make_targets(&self.device, scale);
        let (present_sampler, linear_sampler) = make_samplers(&self.device, scale);
        let water_bind_group = make_water_bind_group(
            &self.device,
            &self.water_layout,
            &underwater,
            &disturbance,
            &linear_sampler,
            &self._water_uniforms,
            &self._ripple_buffer,
        );
        let weather_bind_group = make_weather_bind_group(
            &self.device,
            &self.weather_layout,
            &composite,
            &present_sampler,
            &self._weather_uniforms,
        );

        self.scale = scale;
        self.render_width = width;
        self.render_height = height;
        self.underwater = underwater;
        self.composite = composite;
        self.disturbance = disturbance;
        self.present_sampler = present_sampler;
        self.linear_sampler = linear_sampler;
        self.water_bind_group = water_bind_group;
        self.weather_bind_group = weather_bind_group;
        log::info!("render scale {scale}: {width}x{height}");
        Ok(())
    }

    pub fn leaf_sprite_flags(&self) -> [bool; 2] {
        [
            self.leaf_sprite_groups[0].is_some(),
            self.leaf_sprite_groups[1].is_some(),
        ]
    }

    pub fn has_koi_sprite(&self, family: usize) -> bool {
        self.koi_sprite_groups
            .get(family)
            .is_some_and(Option::is_some)
    }

    pub fn set_skin(&mut self, skin: &Skin) {
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sprite sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        for (index, slot) in skin.koi.iter().enumerate() {
            self.koi_sprite_groups[index] = slot
                .as_ref()
                .and_then(|sprite| self.make_sprite_group(sprite, &sampler));
        }
        for (index, slot) in skin.leaf.iter().enumerate() {
            self.leaf_sprite_groups[index] = slot
                .as_ref()
                .and_then(|sprite| self.make_sprite_group(sprite, &sampler));
        }
    }

    fn make_sprite_group(
        &self,
        sprite: &DecodedSprite,
        sampler: &wgpu::Sampler,
    ) -> Option<wgpu::BindGroup> {
        if sprite.width == 0 || sprite.height == 0 {
            return None;
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sprite texture"),
            size: wgpu::Extent3d {
                width: sprite.width,
                height: sprite.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &sprite.rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(sprite.width * 4),
                rows_per_image: Some(sprite.height),
            },
            wgpu::Extent3d {
                width: sprite.width,
                height: sprite.height,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite bind group"),
            layout: &self.sprite_texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        }))
    }

    fn encode_pond(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        time: f32,
        ripple_data: &[[f32; 4]; 64],
        ripple_count: u32,
        fish: FishFrame<'_>,
        surface: SurfaceFrame<'_>,
        sprites: SpriteFrame<'_>,
        weather: &WeatherParams,
    ) {
        let delta = (time - self.last_time).clamp(0.0, 0.1);
        self.last_time = time;
        let speeds = [
            weather.water.large_speed,
            weather.water.secondary_speed,
            weather.water.detail_speed,
        ];
        for (layer, speed) in self.current_times.iter_mut().zip(speeds) {
            *layer += delta * speed;
        }
        let water = &weather.water;
        self.queue.write_buffer(
            &self._water_uniforms,
            0,
            bytemuck::bytes_of(&[
                [POND_WIDTH as f32, POND_HEIGHT as f32, time, ripple_count as f32],
                [0.0, 1.0, 0.015, 0.0],
                [water.tint[0], water.tint[1], water.tint[2], 1.0],
                [water.large[0], water.large[1], water.large[2], 1.0],
                [water.large_core[0], water.large_core[1], water.large_core[2], 1.0],
                [water.secondary[0], water.secondary[1], water.secondary[2], 1.0],
                [
                    water.secondary_core[0],
                    water.secondary_core[1],
                    water.secondary_core[2],
                    1.0,
                ],
                [water.detail[0], water.detail[1], water.detail[2], 1.0],
                [water.detail_core[0], water.detail_core[1], water.detail_core[2], 1.0],
                [water.large_cell, water.secondary_cell, water.detail_cell, 0.0],
                [water.large_opacity, water.secondary_opacity, water.detail_opacity, 0.0],
                [0.94, 0.34, 0.0, 0.0],
                [-0.38, 0.92, 0.0, 0.0],
                [0.71, 0.71, 0.0, 0.0],
                [22.0, 31.0, 59.0, 0.0],
                [0.92, 0.51, -6.38, 0.0],
                [1.2, 0.55, 0.28, 0.0],
                [self.current_times[0], self.current_times[1], self.current_times[2], 0.0],
                [2.2, 4.0, 62.0, 5.55],
                [1.25, 0.8, 30.0, 4.8],
                [0.9, 1.2, 17.0, 10.4],
                [0.3, 0.58, 0.18, 0.0],
                [0.62, 0.34, 0.18, 0.0],
                [0.7, 0.25, 0.45, 0.0],
            ]),
        );
        self.queue.write_buffer(
            &self._bed_uniforms,
            0,
            bytemuck::bytes_of(&BedUniforms {
                resolution: [POND_WIDTH as f32, POND_HEIGHT as f32],
                vertical_tone: weather.bed.vertical_tone,
                grain_scale: 0.54,
                edge_darkening: weather.bed.edge_darkening,
                pad0: 0.0,
                pad1: 0.0,
                pad2: 0.0,
                deep: [
                    weather.bed.deep[0],
                    weather.bed.deep[1],
                    weather.bed.deep[2],
                    1.0,
                ],
                shallow: [
                    weather.bed.shallow[0],
                    weather.bed.shallow[1],
                    weather.bed.shallow[2],
                    1.0,
                ],
                speck: [0.02, 0.065, 0.04, 1.0],
            }),
        );
        self.queue.write_buffer(
            &self._weather_uniforms,
            0,
            bytemuck::bytes_of(&WeatherUniforms {
                time,
                brightness: weather.brightness,
                contrast: weather.contrast,
                saturation: weather.saturation,
                vignette: weather.vignette,
                cloud_strength: weather.cloud_strength,
                light_strength: weather.light_strength,
                pad0: 0.0,
                light_direction: weather.light_direction,
                pad1: [0.0, 0.0],
                tint: [weather.tint[0], weather.tint[1], weather.tint[2], 1.0],
                light_color: [
                    weather.light_color[0],
                    weather.light_color[1],
                    weather.light_color[2],
                    1.0,
                ],
            }),
        );

        self.queue.write_buffer(
            &self.fish_uniforms,
            0,
            bytemuck::bytes_of(&FishUniforms {
                resolution: [POND_WIDTH as f32, POND_HEIGHT as f32],
                pad0: [0.0, 0.0],
                shadow_color: [
                    weather.shadow_color[0],
                    weather.shadow_color[1],
                    weather.shadow_color[2],
                    1.0,
                ],
                shadow_opacity: 1.0,
                pad1: 0.0,
                pad2: 0.0,
                pad3: 0.0,
            }),
        );
        self.queue
            .write_buffer(&self._ripple_buffer, 0, bytemuck::cast_slice(ripple_data));
        self.queue.write_buffer(
            &self.disturbance_uniforms,
            0,
            bytemuck::bytes_of(&DisturbanceUniforms {
                resolution: [POND_WIDTH as f32, POND_HEIGHT as f32],
                time,
                strength: 2.3,
                wave_frequency: 8.5,
                wave_speed: 2.4,
                pad0: 0.0,
                pad1: 0.0,
            }),
        );
        if !fish.outer.is_empty() {
            self.queue
                .write_buffer(&self.outer_buffer, 0, bytemuck::cast_slice(fish.outer));
        }
        if !fish.body.is_empty() {
            self.queue
                .write_buffer(&self.body_buffer, 0, bytemuck::cast_slice(fish.body));
        }
        if !fish.shadow.is_empty() {
            self.queue
                .write_buffer(&self.shadow_buffer, 0, bytemuck::cast_slice(fish.shadow));
        }
        if !fish.disturbance.is_empty() {
            self.queue.write_buffer(
                &self.disturbance_buffer,
                0,
                bytemuck::cast_slice(fish.disturbance),
            );
        }
        for (buffer, vertices) in [
            (&self.lotus_shadow_buffer, surface.shadow),
            (&self.lotus_leaf_buffer, surface.leaves),
            (&self.lotus_vein_buffer, surface.veins),
            (&self.lotus_flower_buffer, surface.flowers),
            (&self.duckweed_shadow_buffer, surface.duckweed_shadow),
            (&self.duckweed_leaf_buffer, surface.duckweed_leaves),
            (&self.duckweed_highlight_buffer, surface.duckweed_highlights),
            (&self.tiny_shadow_buffer, surface.tiny_shadow),
            (&self.tiny_shape_buffer, surface.tiny_shapes),
            (&self.tiny_detail_buffer, surface.tiny_details),
            (&self.butterfly_shadow_buffer, surface.butterfly_shadow),
            (&self.butterfly_buffer, surface.butterflies),
        ] {
            if !vertices.is_empty() {
                self.queue
                    .write_buffer(buffer, 0, bytemuck::cast_slice(vertices));
            }
        }

        for (family, buffer) in self.koi_sprite_buffers.iter().enumerate() {
            let vertices = &sprites.koi[family];
            if !vertices.is_empty() {
                self.queue
                    .write_buffer(buffer, 0, bytemuck::cast_slice(vertices));
            }
        }
        for (palette, buffer) in self.leaf_sprite_buffers.iter().enumerate() {
            let vertices = &sprites.leaves[palette];
            if !vertices.is_empty() {
                self.queue
                    .write_buffer(buffer, 0, bytemuck::cast_slice(vertices));
            }
        }

        // Pond bed into the underwater buffer.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("underwater"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.underwater.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self._bed_pipeline);
            pass.set_bind_group(0, &self._bed_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        if !surface.shadow.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("lotus shadows"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.underwater.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.fish_shadow_pipeline);
            pass.set_bind_group(0, &self.lotus_shadow_bind_group, &[]);
            pass.set_vertex_buffer(0, self.lotus_shadow_buffer.slice(..));
            pass.draw(0..surface.shadow.len() as u32, 0..1);
        }

        if !surface.tiny_shadow.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("tiny fish shadows"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.underwater.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.fish_shadow_pipeline);
            pass.set_bind_group(0, &self.tiny_shadow_bind_group, &[]);
            pass.set_vertex_buffer(0, self.tiny_shadow_buffer.slice(..));
            pass.draw(0..surface.tiny_shadow.len() as u32, 0..1);
        }

        // Fish shadows, then the koi body batches, all over the bed.
        if !fish.shadow.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("fish shadows"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.underwater.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.fish_shadow_pipeline);
            pass.set_bind_group(0, &self.fish_bind_group, &[]);
            pass.set_vertex_buffer(0, self.shadow_buffer.slice(..));
            pass.draw(0..fish.shadow.len() as u32, 0..1);
        }
        for (pipeline, buffer, count) in [
            (&self.fish_pipeline, &self.outer_buffer, fish.outer.len()),
            (&self.fish_pipeline, &self.body_buffer, fish.body.len()),
        ] {
            if count == 0 {
                continue;
            }
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("fish"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.underwater.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &self.fish_bind_group, &[]);
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..count as u32, 0..1);
        }

        for (family, group) in self.koi_sprite_groups.iter().enumerate() {
            let Some(group) = group else { continue };
            let vertices = &sprites.koi[family];
            if vertices.is_empty() {
                continue;
            }
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("koi sprites"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.underwater.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.sprite_pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.set_bind_group(1, &self.sprite_uniform_group, &[]);
            pass.set_vertex_buffer(0, self.koi_sprite_buffers[family].slice(..));
            pass.draw(0..vertices.len() as u32, 0..1);
        }

        for (buffer, vertices) in [
            (&self.tiny_shape_buffer, surface.tiny_shapes),
            (&self.tiny_detail_buffer, surface.tiny_details),
        ] {
            if vertices.is_empty() {
                continue;
            }
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("tiny fish"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.underwater.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.fish_pipeline);
            pass.set_bind_group(0, &self.fish_bind_group, &[]);
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..vertices.len() as u32, 0..1);
        }

        // Koi surface disturbances.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("disturbance"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.disturbance.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if !fish.disturbance.is_empty() {
                pass.set_pipeline(&self.disturbance_pipeline);
                pass.set_bind_group(0, &self.disturbance_bind_group, &[]);
                pass.set_vertex_buffer(0, self.disturbance_buffer.slice(..));
                pass.draw(0..fish.disturbance.len() as u32, 0..1);
            }
        }

        // Water surface reads the bed and disturbances, writes the composite.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("water"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.composite.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self._water_pipeline);
            pass.set_bind_group(0, &self.water_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        if !surface.duckweed_shadow.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("duckweed shadows"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.composite.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.fish_shadow_pipeline);
            pass.set_bind_group(0, &self.duckweed_shadow_bind_group, &[]);
            pass.set_vertex_buffer(0, self.duckweed_shadow_buffer.slice(..));
            pass.draw(0..surface.duckweed_shadow.len() as u32, 0..1);
        }

        if !surface.butterfly_shadow.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("butterfly shadows"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.composite.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.fish_shadow_pipeline);
            pass.set_bind_group(0, &self.butterfly_shadow_bind_group, &[]);
            pass.set_vertex_buffer(0, self.butterfly_shadow_buffer.slice(..));
            pass.draw(0..surface.butterfly_shadow.len() as u32, 0..1);
        }

        for (buffer, vertices) in [
            (&self.duckweed_leaf_buffer, surface.duckweed_leaves),
            (&self.duckweed_highlight_buffer, surface.duckweed_highlights),
        ] {
            if vertices.is_empty() {
                continue;
            }
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("duckweed"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.composite.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.fish_pipeline);
            pass.set_bind_group(0, &self.fish_bind_group, &[]);
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..vertices.len() as u32, 0..1);
        }

        // Surface objects sit on the water: leaves, their veins, then flowers.
        if !surface.leaves.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("lotus leaves"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.composite.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.fish_pipeline);
            pass.set_bind_group(0, &self.fish_bind_group, &[]);
            pass.set_vertex_buffer(0, self.lotus_leaf_buffer.slice(..));
            pass.draw(0..surface.leaves.len() as u32, 0..1);
        }
        for (palette, group) in self.leaf_sprite_groups.iter().enumerate() {
            let Some(group) = group else { continue };
            let vertices = &sprites.leaves[palette];
            if vertices.is_empty() {
                continue;
            }
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("leaf sprites"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.composite.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.sprite_pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.set_bind_group(1, &self.sprite_uniform_group, &[]);
            pass.set_vertex_buffer(0, self.leaf_sprite_buffers[palette].slice(..));
            pass.draw(0..vertices.len() as u32, 0..1);
        }

        if !surface.veins.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("lotus veins"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.composite.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.line_pipeline);
            pass.set_bind_group(0, &self.fish_bind_group, &[]);
            pass.set_vertex_buffer(0, self.lotus_vein_buffer.slice(..));
            pass.draw(0..surface.veins.len() as u32, 0..1);
        }

        if !surface.flowers.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("lotus flowers"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.composite.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.fish_pipeline);
            pass.set_bind_group(0, &self.fish_bind_group, &[]);
            pass.set_vertex_buffer(0, self.lotus_flower_buffer.slice(..));
            pass.draw(0..surface.flowers.len() as u32, 0..1);
        }

        if !surface.butterflies.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("butterflies"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.composite.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.fish_pipeline);
            pass.set_bind_group(0, &self.fish_bind_group, &[]);
            pass.set_vertex_buffer(0, self.butterfly_buffer.slice(..));
            pass.draw(0..surface.butterflies.len() as u32, 0..1);
        }
    }

    /// Render the pond offscreen and write it as a PPM for inspection.
    pub fn dump_pond(
        &mut self,
        path: &str,
        time: f32,
        ripple_data: &[[f32; 4]; 64],
        ripple_count: u32,
        fish: FishFrame<'_>,
        surface: SurfaceFrame<'_>,
        sprites: SpriteFrame<'_>,
        weather: &WeatherParams,
    ) -> anyhow::Result<()> {
        let width = self.render_width;
        let height = self.render_height;
        let row_bytes = width * 4;
        let padded_row = row_bytes.div_ceil(256) * 256;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("dump buffer"),
            size: (padded_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let dump_target = Target::new_sized(
            &self.device,
            wgpu::TextureFormat::Rgba8Unorm,
            width,
            height,
            "dump weather",
        );
        self.encode_pond(
            &mut encoder,
            time,
            ripple_data,
            ripple_count,
            fish,
            surface,
            sprites,
            weather,
        );
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("dump weather"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &dump_target.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.weather_pipeline_rgba);
            pass.set_bind_group(0, &self.weather_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &dump_target.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::PollType::wait_indefinitely())?;
        receiver.recv()??;

        let data = slice.get_mapped_range()?;
        let mut ppm = Vec::with_capacity((width * height * 3) as usize + 32);
        ppm.extend_from_slice(format!("P6\n{width} {height}\n255\n").as_bytes());
        for row in 0..height {
            let start = (row * padded_row) as usize;
            for column in 0..width {
                let pixel = &data[start + column as usize * 4..][..3];
                ppm.extend_from_slice(pixel);
            }
        }
        std::fs::write(path, ppm)?;
        Ok(())
    }

    pub fn draw(
        &mut self,
        time: f32,
        ripple_data: &[[f32; 4]; 64],
        ripple_count: u32,
        fish: FishFrame<'_>,
        surface: SurfaceFrame<'_>,
        sprites: SpriteFrame<'_>,
        weather: &WeatherParams,
    ) {
        if self.surface_config.is_none() {
            return;
        }

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            _ => return,
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        self.encode_pond(
            &mut encoder,
            time,
            ripple_data,
            ripple_count,
            fish,
            surface,
            sprites,
            weather,
        );


        // Weather post pass writes the swapchain, nearest-upscaling the pond.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("weather"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self._weather_pipeline);
            pass.set_bind_group(0, &self.weather_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
    }

}
