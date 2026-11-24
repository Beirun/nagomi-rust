//! Port of fish-renderer.ts's procedural koi geometry.

use bytemuck::{Pod, Zeroable};

use crate::sim::{
    from_angle, vec2, FishConfig, Koi, Vec2, SPINE_NODES,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 3],
}

impl Vertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: 8,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
            },
        ],
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SpriteVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub tint: [f32; 3],
}

impl SpriteVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: 8,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: 16,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x3,
            },
        ],
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct DisturbanceVertex {
    pub center: [f32; 2],
    pub direction: [f32; 2],
    pub size: [f32; 2],
    pub depth: [f32; 2],
    pub local: [f32; 2],
}

impl DisturbanceVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<DisturbanceVertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: 8,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: 16,
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: 24,
                shader_location: 3,
                format: wgpu::VertexFormat::Float32x2,
            },
            wgpu::VertexAttribute {
                offset: 32,
                shader_location: 4,
                format: wgpu::VertexFormat::Float32x2,
            },
        ],
    };
}

#[derive(Default)]
pub struct Batch {
    pub vertices: Vec<Vertex>,
}

impl Batch {
    pub fn reset(&mut self) {
        self.vertices.clear();
    }

    pub fn point(&mut self, point: Vec2, color: [f32; 3]) {
        self.vertices.push(Vertex {
            position: [point.x, point.y],
            color,
        });
    }

    pub fn triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, color: [f32; 3]) {
        self.point(a, color);
        self.point(b, color);
        self.point(c, color);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn circle(
        &mut self,
        center: Vec2,
        radius: f32,
        color: [f32; 3],
        segments: usize,
    ) {
        for index in 0..segments {
            let angle_a = index as f32 / segments as f32 * std::f32::consts::TAU;
            let angle_b = (index + 1) as f32 / segments as f32 * std::f32::consts::TAU;
            self.triangle(
                center,
                center.add(from_angle(angle_a).mul(radius)),
                center.add(from_angle(angle_b).mul(radius)),
                color,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn ellipse(
        &mut self,
        center: Vec2,
        forward: Vec2,
        normal: Vec2,
        forward_radius: f32,
        side_radius: f32,
        color: [f32; 3],
        phase: f32,
        segments: usize,
    ) {
        let point_at = |angle: f32| {
            let wobble = 1.0
                + (angle * 3.0 + phase).sin() * 0.08
                + (angle * 2.0 - phase * 0.7).cos() * 0.045;
            center
                .add(forward.mul(angle.cos() * forward_radius * wobble))
                .add(normal.mul(angle.sin() * side_radius * wobble))
        };
        for index in 0..segments {
            let angle_a = index as f32 / segments as f32 * std::f32::consts::TAU;
            let angle_b = (index + 1) as f32 / segments as f32 * std::f32::consts::TAU;
            self.triangle(center, point_at(angle_a), point_at(angle_b), color);
        }
    }
}

const fn srgb(hex: u32) -> [f32; 3] {
    [
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
    ]
}

pub struct Palette {
    pub base: [f32; 3],
    pub accent: [f32; 3],
    pub marking: [f32; 3],
    pub fin: [f32; 3],
}

pub const KOI_PALETTES: [Palette; 6] = [
    Palette { base: srgb(0xf1eadb), accent: srgb(0xdc4b2f), marking: srgb(0x27251f), fin: srgb(0xe6ddca) },
    Palette { base: srgb(0xf2ebdc), accent: srgb(0xdf5032), marking: srgb(0x20211f), fin: srgb(0xe7dece) },
    Palette { base: srgb(0xeee6d5), accent: srgb(0xd9482e), marking: srgb(0x242622), fin: srgb(0xc9bfaa) },
    Palette { base: srgb(0xe7aa31), accent: srgb(0xcf7626), marking: srgb(0x78431f), fin: srgb(0xd9922a) },
    Palette { base: srgb(0xf2ebdc), accent: srgb(0xda4430), marking: srgb(0x292723), fin: srgb(0xe7dece) },
    Palette { base: srgb(0xeae5da), accent: srgb(0x252825), marking: srgb(0x4f5c5a), fin: srgb(0xd7d2c7) },
];

const EYE_COLOR: [f32; 3] = srgb(0x171815);

#[derive(Clone, Copy)]
pub struct Patch {
    pub position: f32,
    pub length: f32,
    pub width: f32,
    pub offset: f32,
    pub marking: bool,
}

pub const KOI_PATTERN_PATCHES: [&[Patch]; 6] = [
    &[
        Patch { position: 0.17, length: 0.09, width: 0.74, offset: 0.04, marking: false },
        Patch { position: 0.48, length: 0.115, width: 0.69, offset: -0.12, marking: false },
        Patch { position: 0.76, length: 0.085, width: 0.62, offset: 0.16, marking: false },
    ],
    &[
        Patch { position: 0.19, length: 0.095, width: 0.7, offset: 0.04, marking: false },
        Patch { position: 0.58, length: 0.105, width: 0.66, offset: -0.14, marking: false },
        Patch { position: 0.38, length: 0.045, width: 0.3, offset: 0.38, marking: true },
        Patch { position: 0.79, length: 0.04, width: 0.28, offset: -0.36, marking: true },
    ],
    &[
        Patch { position: 0.15, length: 0.082, width: 0.63, offset: -0.05, marking: false },
        Patch { position: 0.53, length: 0.092, width: 0.58, offset: 0.17, marking: false },
        Patch { position: 0.32, length: 0.09, width: 0.72, offset: 0.1, marking: true },
        Patch { position: 0.73, length: 0.105, width: 0.67, offset: -0.14, marking: true },
    ],
    &[],
    &[Patch { position: 0.16, length: 0.07, width: 0.52, offset: 0.0, marking: false }],
    &[
        Patch { position: 0.22, length: 0.092, width: 0.68, offset: 0.08, marking: false },
        Patch { position: 0.51, length: 0.09, width: 0.6, offset: -0.18, marking: false },
        Patch { position: 0.79, length: 0.074, width: 0.54, offset: 0.22, marking: false },
    ],
];

pub struct Appearance {
    pub pattern: usize,
    pub base: [f32; 3],
    pub accent: [f32; 3],
    pub marking: [f32; 3],
    pub fin: [f32; 3],
    pub eye: [f32; 3],
}

pub fn appearance_for(index: usize) -> Appearance {
    let pattern = index % KOI_PALETTES.len();
    let palette = &KOI_PALETTES[pattern];
    Appearance {
        pattern,
        base: palette.base,
        accent: palette.accent,
        marking: palette.marking,
        fin: palette.fin,
        eye: EYE_COLOR,
    }
}

pub fn visual_depth(config: &FishConfig, depth: f32) -> f32 {
    let range = (config.depth_visual_end - config.depth_visual_start).max(0.001);
    let linear = clamp01((depth - config.depth_visual_start) / range);
    linear * linear * (3.0 - 2.0 * linear)
}

fn clamp01(value: f32) -> f32 {
    value.max(0.0).min(1.0)
}

fn apply_depth_color(source: [f32; 3], config: &FishConfig, depth: f32) -> [f32; 3] {
    let brightness = 1.0 + (config.deep_brightness - 1.0) * depth;
    let saturation = 1.0 + (config.deep_saturation - 1.0) * depth;
    let luminance = source[0] * 0.2126 + source[1] * 0.7152 + source[2] * 0.0722;
    let tint = config.deep_water_tint;
    [
        (luminance + (source[0] - luminance) * saturation) * brightness * (1.0 + (tint[0] - 1.0) * depth),
        (luminance + (source[1] - luminance) * saturation) * brightness * (1.0 + (tint[1] - 1.0) * depth),
        (luminance + (source[2] - luminance) * saturation) * brightness * (1.0 + (tint[2] - 1.0) * depth),
    ]
}

pub fn depth_appearance(source: &Appearance, config: &FishConfig, depth: f32) -> Appearance {
    Appearance {
        pattern: source.pattern,
        base: apply_depth_color(source.base, config, depth),
        accent: apply_depth_color(source.accent, config, depth),
        marking: apply_depth_color(source.marking, config, depth),
        fin: apply_depth_color(source.fin, config, depth),
        eye: apply_depth_color(source.eye, config, depth),
    }
}

/// Depth tint applied to a sprite, matching the koi depth shading.
pub fn depth_tint(config: &FishConfig, depth: f32) -> [f32; 3] {
    let visual = visual_depth(config, depth);
    let brightness = 1.0 + (config.deep_brightness - 1.0) * visual;
    let tint = config.deep_water_tint;
    [
        brightness * (1.0 + (tint[0] - 1.0) * visual),
        brightness * (1.0 + (tint[1] - 1.0) * visual),
        brightness * (1.0 + (tint[2] - 1.0) * visual),
    ]
}

/// Rotated quad for one sprite. Art is assumed to face right; leftward motion
/// flips it vertically so creatures stay upright.
#[allow(clippy::too_many_arguments)]
pub fn push_sprite(
    out: &mut Vec<SpriteVertex>,
    center: Vec2,
    forward: Vec2,
    width: f32,
    height: f32,
    tint: [f32; 3],
) {
    let side = forward.perpendicular();
    let flip = forward.x < 0.0;
    let half_width = width * 0.5;
    let half_height = height * 0.5;
    for (cx, cy) in [
        (-1.0f32, -1.0f32),
        (1.0, -1.0),
        (1.0, 1.0),
        (-1.0, -1.0),
        (1.0, 1.0),
        (-1.0, 1.0),
    ] {
        let point = center
            .add(forward.mul(cx * half_width))
            .add(side.mul(cy * half_height));
        let u = cx * 0.5 + 0.5;
        let mut v = cy * 0.5 + 0.5;
        if flip {
            v = 1.0 - v;
        }
        out.push(SpriteVertex {
            position: [point.x, point.y],
            uv: [u, v],
            tint,
        });
    }
}

pub struct KoiDraw {
    pub outer: Batch,
    pub body: Batch,
    pub shadow: Batch,
    pub disturbance: Vec<DisturbanceVertex>,
}

impl KoiDraw {
    pub fn new() -> Self {
        Self {
            outer: Batch::default(),
            body: Batch::default(),
            shadow: Batch::default(),
            disturbance: Vec::with_capacity(MAX_FISH_VERTICES),
        }
    }

    pub fn clear(&mut self) {
        self.outer.reset();
        self.body.reset();
        self.shadow.reset();
        self.disturbance.clear();
    }
}

const MAX_FISH_VERTICES: usize = crate::sim::MAX_FISH * 6;

struct FishGeometry {
    shadow_offset: Vec2,
    shadow_opacity: f32,
}

fn width_at(fish: &Koi, node: usize) -> f32 {
    let t = node as f32 / (SPINE_NODES - 1) as f32;
    let profile = if t < 0.18 {
        0.73 + (t / 0.18) * 0.27
    } else {
        (1.0 - (t - 0.18) / 0.82).max(0.0).powf(0.72)
    };
    (fish.body_width * profile).max(0.7)
}

pub fn build_render_spine(fish: &mut Koi) {
    fish.render_spine[0] = fish.spine[0];
    for node in 1..SPINE_NODES {
        let t = node as f32 / (SPINE_NODES - 1) as f32;
        let previous = node.saturating_sub(1);
        let next = (node + 1).min(SPINE_NODES - 1);
        let tangent = fish.spine[previous]
            .sub(fish.spine[next])
            .normalize_or(from_angle(fish.heading));
        let normal = tangent.perpendicular();
        let wave_envelope = t.powf(1.72);
        let wave = (fish.swim_phase - t * 6.1).sin()
            * fish.body_width
            * 1.15
            * wave_envelope
            * (0.08 + fish.tail_effort * 0.92);
        fish.render_spine[node] = fish.spine[node].add(normal.mul(wave));
    }

    // Light smoothing rounds the remaining corners between joints without
    // moving the head or the tail tip.
    for _ in 0..2 {
        for node in 1..SPINE_NODES - 1 {
            let neighbours = fish.render_spine[node - 1].add(fish.render_spine[node + 1]);
            fish.render_spine[node] = fish.render_spine[node]
                .mul(0.6)
                .add(neighbours.mul(0.2));
        }
    }
}

fn silhouette_triangle(
    geometry: &FishGeometry,
    draw: &mut KoiDraw,
    a: Vec2,
    b: Vec2,
    c: Vec2,
    color: [f32; 3],
) {
    let strength = geometry.shadow_opacity;
    draw.shadow.triangle(
        a.add(geometry.shadow_offset),
        b.add(geometry.shadow_offset),
        c.add(geometry.shadow_offset),
        [strength, strength, strength],
    );
    draw.outer.triangle(a, b, c, color);
}

fn silhouette_circle(
    geometry: &FishGeometry,
    draw: &mut KoiDraw,
    center: Vec2,
    radius: f32,
    color: [f32; 3],
) {
    let strength = geometry.shadow_opacity;
    draw.shadow.circle(
        center.add(geometry.shadow_offset),
        radius,
        [strength, strength, strength],
        12,
    );
    draw.outer.circle(center, radius, color, 12);
}

pub fn draw_koi(
    config: &FishConfig,
    fish: &Koi,
    appearance: &Appearance,
    draw: &mut KoiDraw,
) {
    let depth = visual_depth(config, fish.depth);
    let shadow_offset = config.shadow_offset.add(config.shadow_depth_offset.mul(depth));
    let shadow_opacity = config.shadow_surface_opacity
        + (config.shadow_deep_opacity - config.shadow_surface_opacity) * depth;
    let geometry = FishGeometry {
        shadow_offset,
        shadow_opacity,
    };

    let mut left = [Vec2::default(); SPINE_NODES];
    let mut right = [Vec2::default(); SPINE_NODES];
    for node in 0..SPINE_NODES {
        let previous = node.saturating_sub(1);
        let next = (node + 1).min(SPINE_NODES - 1);
        let tangent = fish.render_spine[previous]
            .sub(fish.render_spine[next])
            .normalize_or(from_angle(fish.heading));
        let normal = tangent.perpendicular();
        let half_width = width_at(fish, node);
        left[node] = fish.render_spine[node].add(normal.mul(half_width));
        right[node] = fish.render_spine[node].add(normal.mul(-half_width));
    }

    let pectoral_front = 3;
    let pectoral_center = 4;
    let pectoral_back = 6;
    let pectoral_tangent = fish.render_spine[pectoral_center - 1]
        .sub(fish.render_spine[pectoral_center + 1])
        .normalize_or(from_angle(fish.heading));
    let pectoral_normal = pectoral_tangent.perpendicular();
    let gulp_progress = fish.gulp_animation / config.feeding_animation_seconds.max(0.001);
    let gulp_paddle = (std::f32::consts::PI * gulp_progress).sin() * 0.85;
    let paddle_activity = match fish.state {
        crate::sim::SwimState::Hover => 1.0,
        crate::sim::SwimState::Pivot => 0.85,
        _ => 0.45,
    } + gulp_paddle;
    let fin_pulse =
        0.82 + paddle_activity * 0.25 * (fish.swim_phase * 0.64 + fish.phase_offset).sin();
    let pectoral_reach = fish.body_width * (0.55 + paddle_activity * 0.25) * fin_pulse;
    let left_pectoral = left[pectoral_center]
        .add(pectoral_normal.mul(pectoral_reach))
        .add(pectoral_tangent.mul(-fish.body_width * 0.22));
    let right_pectoral = right[pectoral_center]
        .add(pectoral_normal.mul(-pectoral_reach))
        .add(pectoral_tangent.mul(-fish.body_width * 0.22));
    silhouette_triangle(
        &geometry,
        draw,
        left[pectoral_front],
        left_pectoral,
        left[pectoral_back],
        appearance.fin,
    );
    silhouette_triangle(
        &geometry,
        draw,
        right[pectoral_front],
        right[pectoral_back],
        right_pectoral,
        appearance.fin,
    );

    let pelvic_front = 7;
    let pelvic_center = 8;
    let pelvic_back = 9;
    let pelvic_tangent = fish.render_spine[pelvic_center - 1]
        .sub(fish.render_spine[pelvic_center + 1])
        .normalize_or(from_angle(fish.heading));
    let pelvic_normal = pelvic_tangent.perpendicular();
    let pelvic_reach = fish.body_width * (0.28 + 0.05 * fin_pulse);
    let left_pelvic = left[pelvic_center].add(pelvic_normal.mul(pelvic_reach));
    let right_pelvic = right[pelvic_center].add(pelvic_normal.mul(-pelvic_reach));
    silhouette_triangle(
        &geometry,
        draw,
        left[pelvic_front],
        left_pelvic,
        left[pelvic_back],
        appearance.fin,
    );
    silhouette_triangle(
        &geometry,
        draw,
        right[pelvic_front],
        right[pelvic_back],
        right_pelvic,
        appearance.fin,
    );

    for node in (0..SPINE_NODES - 1).rev() {
        silhouette_triangle(
            &geometry,
            draw,
            left[node],
            right[node],
            right[node + 1],
            appearance.base,
        );
        silhouette_triangle(
            &geometry,
            draw,
            left[node],
            right[node + 1],
            left[node + 1],
            appearance.base,
        );
    }

    let head_forward = fish.render_spine[0]
        .sub(fish.render_spine[1])
        .normalize_or(from_angle(fish.heading));
    let head_normal = head_forward.perpendicular();
    let nose_center = fish.render_spine[0].add(head_forward.mul(fish.body_width * 0.43));
    let nose_half_width = width_at(fish, 0) * 0.72;
    let nose_left = nose_center.add(head_normal.mul(nose_half_width));
    let nose_right = nose_center.add(head_normal.mul(-nose_half_width));
    silhouette_triangle(&geometry, draw, left[0], nose_left, nose_right, appearance.base);
    silhouette_triangle(&geometry, draw, left[0], nose_right, right[0], appearance.base);
    silhouette_circle(
        &geometry,
        draw,
        nose_center,
        (nose_half_width * 0.72).max(1.0),
        appearance.base,
    );

    let tail_node = SPINE_NODES - 1;
    let tail_forward = fish.render_spine[tail_node - 1]
        .sub(fish.render_spine[tail_node])
        .normalize_or(from_angle(fish.heading));
    let tail_normal = tail_forward.perpendicular();
    let tail_backward = tail_forward.mul(-1.0);
    let tail_spread = fish.body_width * (0.58 + 0.08 * (fish.swim_phase - 0.8).sin());
    let upper_fin = fish.render_spine[tail_node]
        .add(tail_backward.mul(fish.body_width * 1.38))
        .add(tail_normal.mul(tail_spread));
    let lower_fin = fish.render_spine[tail_node]
        .add(tail_backward.mul(fish.body_width * 1.38))
        .add(tail_normal.mul(-tail_spread));
    let tail_notch = fish.render_spine[tail_node].add(tail_backward.mul(fish.body_width * 0.86));
    silhouette_triangle(
        &geometry,
        draw,
        fish.render_spine[tail_node],
        upper_fin,
        tail_notch,
        appearance.fin,
    );
    silhouette_triangle(
        &geometry,
        draw,
        fish.render_spine[tail_node],
        tail_notch,
        lower_fin,
        appearance.fin,
    );

    for (patch_index, patch) in KOI_PATTERN_PATCHES[appearance.pattern].iter().enumerate() {
        let spine_position = patch.position * (SPINE_NODES - 1) as f32;
        let node = ((SPINE_NODES - 2).min(spine_position.floor() as usize)).max(0);
        let amount = spine_position - node as f32;
        let center = fish.render_spine[node].lerp(fish.render_spine[node + 1], amount);
        let previous = node.saturating_sub(1);
        let next = (node + 2).min(SPINE_NODES - 1);
        let forward = fish.render_spine[previous]
            .sub(fish.render_spine[next])
            .normalize_or(from_angle(fish.heading));
        let normal = forward.perpendicular();
        let local_width = width_at(fish, node) * (1.0 - amount) + width_at(fish, node + 1) * amount;
        let patch_center = center.add(normal.mul(local_width * patch.offset));
        let patch_color = if patch.marking {
            appearance.marking
        } else {
            appearance.accent
        };
        draw.body.ellipse(
            patch_center,
            forward,
            normal,
            fish.body_length * patch.length,
            local_width * patch.width,
            patch_color,
            patch_index as f32 * 1.73 + patch.position * 5.1,
            10,
        );
    }

    let eye_anchor = fish.render_spine[0].add(head_forward.mul(fish.body_width * 0.08));
    let eye_offset = width_at(fish, 0) * 0.58;
    let eye_radius = (fish.body_width * 0.11).max(0.58);
    draw.body
        .circle(eye_anchor.add(head_normal.mul(eye_offset)), eye_radius, appearance.eye, 6);
    draw.body
        .circle(eye_anchor.add(head_normal.mul(-eye_offset)), eye_radius, appearance.eye, 6);

    // Surface disturbance quad for the water pass.
    let smooth_depth = depth * depth * (3.0 - 2.0 * depth);
    if smooth_depth > 0.005 {
        let center = [fish.position.x, fish.position.y];
        let direction = [fish.heading.cos(), fish.heading.sin()];
        let size = [
            fish.body_length * config.distortion_length_scale,
            fish.body_width * config.distortion_width_scale,
        ];
        let depth_pair = [smooth_depth, fish.phase_offset];
        for corner in [
            [-1.0, -1.0],
            [1.0, -1.0],
            [1.0, 1.0],
            [-1.0, -1.0],
            [1.0, 1.0],
            [-1.0, 1.0],
        ] {
            draw.disturbance.push(DisturbanceVertex {
                center,
                direction,
                size,
                depth: depth_pair,
                local: corner,
            });
        }
    }
}

// ---- built-in creature skin -------------------------------------------------
// Original pond critters drawn from the same polygons as the koi. They keep
// the simulation's motion and depth shading.

const CREATURE_PALETTES: [[[u32; 3]; 3]; 6] = [
    // axolotl: body, belly, gills
    [[0xf4, 0xa3, 0xc8], [0xff, 0xd1, 0xe6], [0xe8, 0x62, 0x9a]],
    // turtle: shell, shell shade, skin
    [[0x4f, 0x9d, 0x6b], [0x3c, 0x7a, 0x53], [0x7f, 0xbf, 0x8e]],
    // frog: body, light, dark
    [[0x79, 0xc2, 0x5c], [0xa8, 0xdd, 0x7f], [0x4e, 0x8a, 0x3c]],
    // duck: body, wing, beak
    [[0xf2, 0xf0, 0xe6], [0xd9, 0xd4, 0xc4], [0xe8, 0xa3, 0x3d]],
    // puffer: body, belly, spikes
    [[0xf3, 0xc4, 0x53], [0xff, 0xe0, 0x8a], [0xd9, 0x9a, 0x2b]],
    // newt: body, belly, spots
    [[0x5a, 0xa9, 0xc9], [0xa6, 0xd9, 0xea], [0x3f, 0x7f, 0x99]],
];

fn palette_rgb(value: [u32; 3]) -> [f32; 3] {
    [
        value[0] as f32 / 255.0,
        value[1] as f32 / 255.0,
        value[2] as f32 / 255.0,
    ]
}

pub fn draw_creature(config: &FishConfig, fish: &Koi, family: usize, draw: &mut KoiDraw) {
    let depth = visual_depth(config, fish.depth);
    let tint = |value: [u32; 3]| apply_depth_color(palette_rgb(value), config, depth);
    let palette = CREATURE_PALETTES[family % CREATURE_PALETTES.len()];
    let body = tint(palette[0]);
    let second = tint(palette[1]);
    let accent = tint(palette[2]);
    let eye = tint([0x1f, 0x21, 0x24]);
    let white = tint([0xf7, 0xf3, 0xe8]);

    let forward = from_angle(fish.heading);
    let side = forward.perpendicular();
    let head = fish.render_spine[0];
    let tail = fish.render_spine[SPINE_NODES - 1];
    let center = head.add(tail).mul(0.5);
    let length = fish.body_length;
    let width = fish.body_width;

    let at = |along: f32, across: f32| {
        head.add(forward.mul(-along * length))
            .add(side.mul(across * width))
    };
    let shadow_strength = (config.shadow_surface_opacity
        + (config.shadow_deep_opacity - config.shadow_surface_opacity) * depth)
        * 0.85;
    let shadow_offset = config
        .shadow_offset
        .add(config.shadow_depth_offset.mul(depth));
    draw.shadow.circle(
        center.add(shadow_offset),
        length * 0.2,
        [shadow_strength, shadow_strength, shadow_strength],
        14,
    );

    match family % 6 {
        0 => {
            // Axolotl
            draw.outer.ellipse(
                at(0.44, 0.0),
                forward,
                side,
                length * 0.4,
                width * 0.9,
                body,
                fish.phase_offset,
                14,
            );
            for index in 0..3 {
                let along = 0.16 + index as f32 * 0.07;
                let spread = 1.5 + index as f32 * 0.28;
                for direction in [-1.0f32, 1.0] {
                    let angle = fish.heading + direction * spread;
                    let gill = vec2(angle.cos(), angle.sin());
                    let root = at(along, direction * 0.45);
                    draw.outer.ellipse(
                        root.add(gill.mul(width * 0.6)),
                        gill,
                        gill.perpendicular(),
                        width * 0.65,
                        width * 0.2,
                        accent,
                        0.0,
                        8,
                    );
                }
            }
            draw.outer.ellipse(
                at(0.72, 0.0),
                forward,
                side,
                length * 0.26,
                width * 0.4,
                second,
                fish.phase_offset,
                10,
            );
            draw.outer.circle(at(0.02, 0.0), width * 0.8, body, 12);
            draw.outer.ellipse(
                at(0.45, 0.0),
                forward,
                side,
                length * 0.18,
                width * 0.4,
                second,
                0.0,
                8,
            );
            for direction in [-1.0f32, 1.0] {
                draw.outer.circle(at(0.12, direction * 0.36), width * 0.17, eye, 6);
            }
        }
        1 => {
            // Turtle: flippers first so the shell sits over their roots.
            for (along, direction) in [
                (0.22, -1.0f32),
                (0.22, 1.0),
                (0.68, -1.0),
                (0.68, 1.0),
            ] {
                let angle = fish.heading
                    + direction * if along < 0.5 { 0.9 } else { 2.3 };
                let limb = vec2(angle.cos(), angle.sin());
                let root = at(along, direction * 0.6);
                draw.outer.ellipse(
                    root.add(limb.mul(width * 0.55)),
                    forward,
                    side,
                    width * 0.55,
                    width * 0.28,
                    accent,
                    0.0,
                    8,
                );
            }
            draw.outer.ellipse(
                at(0.42, 0.0),
                forward,
                side,
                length * 0.4,
                width * 1.1,
                second,
                0.0,
                14,
            );
            draw.outer.ellipse(
                at(0.42, 0.0),
                forward,
                side,
                length * 0.33,
                width * 0.92,
                body,
                0.0,
                14,
            );
            for index in 0..3 {
                let along = 0.26 + index as f32 * 0.13;
                draw.outer.circle(at(along, 0.0), width * 0.2, second, 6);
            }
            draw.outer.circle(at(0.02, 0.0), width * 0.58, accent, 10);
            for direction in [-1.0f32, 1.0] {
                draw.outer.circle(at(0.08, direction * 0.24), width * 0.13, eye, 5);
            }
        }
        2 => {
            // Frog: hind legs behind the body, prominent eyes on top.
            for direction in [-1.0f32, 1.0] {
                let hip = at(0.7, direction * 0.65);
                let ankle = at(0.55, direction * 1.4);
                draw.outer.ellipse(
                    hip.add(ankle).mul(0.5),
                    ankle.sub(hip).normalize_or(side),
                    ankle.sub(hip).normalize_or(side).perpendicular(),
                    hip.sub(ankle).length() * 0.62,
                    width * 0.32,
                    second,
                    0.0,
                    8,
                );
                draw.outer.ellipse(
                    at(0.32, direction * 0.95),
                    forward,
                    side,
                    length * 0.07,
                    width * 0.22,
                    second,
                    0.0,
                    6,
                );
            }
            draw.outer.ellipse(
                at(0.42, 0.0),
                forward,
                side,
                length * 0.36,
                width * 1.05,
                body,
                fish.phase_offset,
                14,
            );
            for direction in [-1.0f32, 1.0] {
                let bump = at(0.1, direction * 0.52);
                draw.outer.circle(bump, width * 0.46, body, 9);
                draw.outer.circle(
                    bump.add(forward.mul(width * 0.08)),
                    width * 0.22,
                    eye,
                    6,
                );
            }
            draw.outer.ellipse(
                at(0.05, 0.0),
                forward,
                side,
                length * 0.03,
                width * 0.4,
                second,
                0.0,
                8,
            );
        }
        3 => {
            // Duck
            draw.outer.ellipse(
                at(0.46, 0.0),
                forward,
                side,
                length * 0.4,
                width * 0.95,
                body,
                fish.phase_offset,
                14,
            );
            draw.outer.triangle(
                at(0.86, 0.0),
                at(1.06, 0.45),
                at(1.06, -0.45),
                second,
            );
            draw.outer.ellipse(
                at(0.5, 0.4),
                forward,
                side,
                length * 0.2,
                width * 0.42,
                second,
                fish.phase_offset,
                10,
            );
            let head_center = at(0.04, 0.0);
            draw.outer.circle(head_center.add(forward.mul(width * 0.16)), width * 0.6, body, 10);
            let beak_root = head_center.add(forward.mul(width * 0.62));
            let beak_tip = beak_root.add(forward.mul(width * 0.75));
            draw.outer.triangle(
                beak_root.add(side.mul(width * 0.26)),
                beak_tip.add(side.mul(width * 0.1)),
                beak_tip.sub(side.mul(width * 0.1)),
                accent,
            );
            draw.outer.triangle(
                beak_root.sub(side.mul(width * 0.26)),
                beak_tip.sub(side.mul(width * 0.1)),
                beak_tip.add(side.mul(width * 0.1)),
                accent,
            );
            draw.outer.circle(
                head_center.add(forward.mul(width * 0.35)).add(side.mul(width * 0.28)),
                width * 0.11,
                eye,
                5,
            );
            draw.outer.circle(
                head_center.add(forward.mul(width * 0.35)).sub(side.mul(width * 0.28)),
                width * 0.11,
                eye,
                5,
            );
        }
        4 => {
            // Puffer: eyes and mouth sit inside the round body.
            draw.outer.circle(at(0.48, 0.0), length * 0.32, body, 16);
            for index in 0..12 {
                let angle = index as f32 / 12.0 * std::f32::consts::TAU;
                let direction = vec2(angle.cos(), angle.sin());
                let root = at(0.48, 0.0).add(direction.mul(length * 0.3));
                draw.outer.circle(root.add(direction.mul(width * 0.14)), width * 0.17, accent, 6);
            }
            draw.outer.ellipse(
                at(0.54, 0.0),
                forward,
                side,
                length * 0.14,
                width * 0.42,
                second,
                0.0,
                10,
            );
            draw.outer.ellipse(
                at(0.98, 0.0),
                forward,
                side,
                length * 0.09,
                width * 0.34,
                second,
                0.0,
                8,
            );
            for direction in [-1.0f32, 1.0] {
                draw.outer.ellipse(
                    at(0.5, direction * 0.8),
                    side,
                    forward,
                    width * 0.3,
                    width * 0.14,
                    accent,
                    0.0,
                    8,
                );
                let eye_center = at(0.3, direction * 0.38);
                draw.outer.circle(eye_center, width * 0.28, white, 8);
                draw.outer.circle(eye_center, width * 0.14, eye, 6);
            }
            draw.outer.circle(at(0.14, 0.0), width * 0.1, eye, 5);
        }
        _ => {
            // Newt: slim body, visible legs, dorsal stripe.
            for (along, direction) in [
                (0.28, -1.0f32),
                (0.28, 1.0),
                (0.62, -1.0),
                (0.62, 1.0),
            ] {
                draw.outer.ellipse(
                    at(along, direction * 0.66),
                    side,
                    forward,
                    width * 0.3,
                    width * 0.13,
                    accent,
                    0.0,
                    6,
                );
            }
            draw.outer.ellipse(
                at(0.44, 0.0),
                forward,
                side,
                length * 0.38,
                width * 0.58,
                body,
                fish.phase_offset,
                12,
            );
            draw.outer.ellipse(
                at(0.5, 0.0),
                forward,
                side,
                length * 0.26,
                width * 0.24,
                second,
                0.0,
                10,
            );
            draw.outer.ellipse(
                at(0.96, 0.0),
                forward,
                side,
                length * 0.22,
                width * 0.4,
                second,
                0.0,
                10,
            );
            draw.outer.circle(at(0.03, 0.0), width * 0.48, body, 10);
            for direction in [-1.0f32, 1.0] {
                draw.outer.circle(at(0.09, direction * 0.24), width * 0.13, eye, 5);
                draw.outer.circle(at(0.42, direction * 0.32), width * 0.07, accent, 4);
            }
        }
    }
}
