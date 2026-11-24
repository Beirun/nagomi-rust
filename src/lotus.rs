//! Port of lotus-leaves.ts: static leaf/flower geometry with per-frame transforms.

use crate::fish::Vertex;
use crate::sim::{vec2, Vec2, XorShift32};

const TAU: f32 = std::f32::consts::TAU;

pub const VISIBLE_LEAF_COUNT: usize = 15;
pub const VISIBLE_FLOWER_COUNT: usize = 4;
pub const MAX_LEAVES: usize = 32;
pub const MAX_FLOWERS: usize = 16;

const RADIUS_SCALE: f32 = 1.18;
const FLOWER_RADIUS_SCALE: f32 = 2.38;
const LEAF_SEGMENTS: usize = 24;
const VEIN_COUNT: usize = 5;
const NOTCH_HALF_ANGLE: f32 = 0.3;
const VERTICAL_SCALE: f32 = 0.92;
const DRIFT_X: f32 = 0.7;
const DRIFT_Y: f32 = 0.55;
const ROTATION_AMOUNT: f32 = 0.055;

const SHADOW_OFFSET: Vec2 = vec2(4.8, 10.4);
pub const SHADOW_COLOR: [f32; 3] = [10.0 / 255.0, 43.0 / 255.0, 38.0 / 255.0];
pub const SHADOW_OPACITY: f32 = 0.5;

const fn rgb(hex: u32) -> [f32; 3] {
    [
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
    ]
}

struct LeafPalette {
    base: [f32; 3],
    light: [f32; 3],
    shade: [f32; 3],
    vein: [f32; 3],
    center: [f32; 3],
}

const LEAF_PALETTES: [LeafPalette; 2] = [
    LeafPalette {
        base: rgb(0x5f9d78),
        light: rgb(0x76aa84),
        shade: rgb(0x487c66),
        vein: rgb(0x3f705e),
        center: rgb(0x4f866b),
    },
    LeafPalette {
        base: rgb(0x568f6f),
        light: rgb(0x6ca17b),
        shade: rgb(0x416f5b),
        vein: rgb(0x386653),
        center: rgb(0x497d63),
    },
];

struct FlowerPalette {
    outer_petal: [f32; 3],
    inner_petal: [f32; 3],
    petal_light: [f32; 3],
    center: [f32; 3],
    center_dark: [f32; 3],
}

const FLOWER_PALETTES: [FlowerPalette; 2] = [
    FlowerPalette {
        outer_petal: rgb(0xf29aaa),
        inner_petal: rgb(0xffc4cc),
        petal_light: rgb(0xffe1e2),
        center: rgb(0xf2bd45),
        center_dark: rgb(0xb96d31),
    },
    FlowerPalette {
        outer_petal: rgb(0xe985ac),
        inner_petal: rgb(0xfab7ce),
        petal_light: rgb(0xffdce6),
        center: rgb(0xf5c64b),
        center_dark: rgb(0xbd7330),
    },
];

#[derive(Clone, Copy)]
struct LeafPlacement {
    position: Vec2,
    radius: f32,
    angle: f32,
    phase: f32,
    palette: usize,
}

const LEAF_PLACEMENTS: [LeafPlacement; 19] = [
    LeafPlacement { position: vec2(-3.0, 37.0), radius: 22.0, angle: 0.35, phase: 0.2, palette: 0 },
    LeafPlacement { position: vec2(76.0, 17.0), radius: 16.0, angle: 2.15, phase: 1.4, palette: 1 },
    LeafPlacement { position: vec2(431.0, 18.0), radius: 23.0, angle: 2.75, phase: 2.2, palette: 0 },
    LeafPlacement { position: vec2(476.0, 88.0), radius: 17.0, angle: 4.25, phase: 3.3, palette: 1 },
    LeafPlacement { position: vec2(460.0, 151.0), radius: 23.0, angle: 0.95, phase: 4.6, palette: 0 },
    LeafPlacement { position: vec2(488.0, 216.0), radius: 20.0, angle: 3.55, phase: 5.4, palette: 1 },
    LeafPlacement { position: vec2(395.0, 252.0), radius: 22.0, angle: 5.3, phase: 0.9, palette: 0 },
    LeafPlacement { position: vec2(113.0, 260.0), radius: 28.0, angle: 4.65, phase: 2.8, palette: 1 },
    LeafPlacement { position: vec2(31.0, 230.0), radius: 19.0, angle: 1.85, phase: 4.1, palette: 0 },
    LeafPlacement { position: vec2(140.0, 10.0), radius: 21.0, angle: 0.7, phase: 5.9, palette: 1 },
    LeafPlacement { position: vec2(330.0, 7.0), radius: 14.0, angle: 3.85, phase: 1.8, palette: 0 },
    LeafPlacement { position: vec2(447.0, 57.0), radius: 20.0, angle: 5.65, phase: 3.8, palette: 1 },
    LeafPlacement { position: vec2(82.0, 76.0), radius: 32.0, angle: 1.25, phase: 4.9, palette: 0 },
    LeafPlacement { position: vec2(414.0, 194.0), radius: 23.0, angle: 4.85, phase: 2.5, palette: 1 },
    LeafPlacement { position: vec2(444.0, 224.0), radius: 20.0, angle: 2.85, phase: 2.5, palette: 1 },
    LeafPlacement { position: vec2(10.0, 165.0), radius: 14.0, angle: 2.55, phase: 2.5, palette: 0 },
    LeafPlacement { position: vec2(451.0, 246.0), radius: 10.0, angle: 0.15, phase: 3.1, palette: 1 },
    LeafPlacement { position: vec2(58.0, 202.0), radius: 15.0, angle: 5.15, phase: 5.0, palette: 0 },
    LeafPlacement { position: vec2(374.0, 31.0), radius: 19.0, angle: 2.25, phase: 1.1, palette: 1 },
];

#[derive(Clone, Copy)]
struct FlowerPlacement {
    leaf_index: usize,
    radius: f32,
    offset: Vec2,
    rotation: f32,
    palette: usize,
}

const FLOWER_PLACEMENTS: [FlowerPlacement; 4] = [
    FlowerPlacement { leaf_index: 12, radius: 5.4, offset: vec2(0.5, -0.5), rotation: 0.25, palette: 0 },
    FlowerPlacement { leaf_index: 13, radius: 5.0, offset: vec2(-0.8, 0.3), rotation: 0.75, palette: 1 },
    FlowerPlacement { leaf_index: 7, radius: 4.8, offset: vec2(2.0, -1.0), rotation: 0.45, palette: 0 },
    FlowerPlacement { leaf_index: 9, radius: 4.5, offset: vec2(-1.0, 0.5), rotation: 0.15, palette: 1 },
];

fn random_int(rng: &mut XorShift32, low: i32, high: i32) -> i32 {
    low + (rng.unit() * (high - low) as f32) as i32
}

fn random_edge_position(rng: &mut XorShift32) -> Vec2 {
    match random_int(rng, 0, 4) {
        0 => vec2(
            random_int(rng, 16, 465) as f32,
            random_int(rng, 8, 43) as f32,
        ),
        1 => vec2(
            random_int(rng, 438, 473) as f32,
            random_int(rng, 16, 255) as f32,
        ),
        2 => vec2(
            random_int(rng, 16, 465) as f32,
            random_int(rng, 228, 263) as f32,
        ),
        _ => vec2(
            random_int(rng, 8, 43) as f32,
            random_int(rng, 16, 255) as f32,
        ),
    }
}

fn generated_leaf(rng: &mut XorShift32) -> LeafPlacement {
    LeafPlacement {
        position: random_edge_position(rng),
        radius: rng.range(12.0, 27.0),
        angle: rng.range(0.0, TAU),
        phase: rng.range(0.0, TAU),
        palette: if rng.unit() < 0.5 { 0 } else { 1 },
    }
}

fn generated_flower(rng: &mut XorShift32) -> FlowerPlacement {
    FlowerPlacement {
        leaf_index: random_int(rng, 0, VISIBLE_LEAF_COUNT as i32) as usize,
        radius: rng.range(4.2, 6.2),
        offset: vec2(rng.range(-2.2, 2.2), rng.range(-2.2, 2.2)),
        rotation: rng.range(0.0, TAU),
        palette: if rng.unit() < 0.5 { 0 } else { 1 },
    }
}

fn edge_point(radius: f32, angle: f32, phase: f32) -> Vec2 {
    let wobble =
        1.0 + (angle * 3.0 + phase).sin() * 0.035 + (angle * 5.0 - phase).cos() * 0.025;
    vec2(
        angle.cos() * radius * wobble,
        angle.sin() * radius * VERTICAL_SCALE * wobble,
    )
}

fn leaf_color(palette: &LeafPalette, angle: f32, phase: f32) -> [f32; 3] {
    let directional = 0.5 + (angle + 2.2).cos() * 0.42;
    let organic = (angle * 3.0 + phase * 0.7).sin() * 0.045;
    let tone = (directional + organic).max(0.0).min(1.0);
    let mix = |a: [f32; 3], b: [f32; 3], t: f32| {
        [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
        ]
    };
    if tone < 0.5 {
        mix(palette.shade, palette.base, tone * 2.0)
    } else {
        mix(palette.base, palette.light, (tone - 0.5) * 2.0)
    }
}

struct LocalLeaf {
    triangles: Vec<Vertex>,
    vein_lines: Vec<Vertex>,
    center_circle: Vec<Vertex>,
    center: Vec2,
    phase: f32,
}

struct LocalFlower {
    triangles: Vec<Vertex>,
}

pub struct LeafSprite {
    pub center: Vec2,
    pub rotation: f32,
    pub size: f32,
    pub palette: usize,
}

pub struct LotusFrame {
    pub shadow: Vec<Vertex>,
    pub leaves: Vec<Vertex>,
    pub veins: Vec<Vertex>,
    pub flowers: Vec<Vertex>,
    pub leaf_sprites: Vec<LeafSprite>,
}

impl LotusFrame {
    pub fn new() -> Self {
        Self {
            shadow: Vec::new(),
            leaves: Vec::new(),
            veins: Vec::new(),
            flowers: Vec::new(),
            leaf_sprites: Vec::new(),
        }
    }
}

pub struct Lotus {
    placements: Vec<LeafPlacement>,
    flower_placements: Vec<FlowerPlacement>,
    leaves: Vec<LocalLeaf>,
    flowers: Vec<LocalFlower>,
    visible_leaves: usize,
    visible_flowers: usize,
}

impl Lotus {
    pub fn new() -> Self {
        let mut rng = XorShift32::new(0x1eaf_5eed);
        let mut placements: Vec<LeafPlacement> = LEAF_PLACEMENTS.to_vec();
        while placements.len() < MAX_LEAVES {
            placements.push(generated_leaf(&mut rng));
        }
        let mut flower_placements: Vec<FlowerPlacement> = FLOWER_PLACEMENTS.to_vec();
        while flower_placements.len() < MAX_FLOWERS {
            flower_placements.push(generated_flower(&mut rng));
        }

        let mut leaves = Vec::new();
        for placement in placements.iter() {
            let palette = &LEAF_PALETTES[placement.palette % LEAF_PALETTES.len()];
            let radius = placement.radius * RADIUS_SCALE;
            let start = placement.angle + NOTCH_HALF_ANGLE;
            let span = TAU - NOTCH_HALF_ANGLE * 2.0;

            let mut triangles = Vec::new();
            for index in 0..LEAF_SEGMENTS {
                let angle_a = start + index as f32 / LEAF_SEGMENTS as f32 * span;
                let angle_b = start + (index + 1) as f32 / LEAF_SEGMENTS as f32 * span;
                let center_color = palette.center;
                let color_a = leaf_color(palette, angle_a, placement.phase);
                let color_b = leaf_color(palette, angle_b, placement.phase);
                let center = Vec2::default();
                let a = edge_point(radius, angle_a, placement.phase);
                let b = edge_point(radius, angle_b, placement.phase);
                triangles.push(Vertex { position: [center.x, center.y], color: center_color });
                triangles.push(Vertex { position: [a.x, a.y], color: color_a });
                triangles.push(Vertex { position: [b.x, b.y], color: color_b });
            }

            let mut center_circle = Vec::new();
            let circle_radius = (radius * 0.075).max(1.0);
            for index in 0..8 {
                let angle_a = index as f32 / 8.0 * TAU;
                let angle_b = (index + 1) as f32 / 8.0 * TAU;
                for p in [
                    Vec2::default(),
                    vec2(angle_a.cos() * circle_radius, angle_a.sin() * circle_radius),
                    vec2(angle_b.cos() * circle_radius, angle_b.sin() * circle_radius),
                ] {
                    center_circle.push(Vertex { position: [p.x, p.y], color: palette.center });
                }
            }

            let mut vein_lines = Vec::new();
            for index in 1..=VEIN_COUNT {
                let vein_angle = start + index as f32 / (VEIN_COUNT + 1) as f32 * span;
                let end = edge_point(radius * 0.68, vein_angle, placement.phase);
                vein_lines.push(Vertex { position: [0.0, 0.0], color: palette.vein });
                vein_lines.push(Vertex { position: [end.x, end.y], color: palette.vein });
            }

            leaves.push(LocalLeaf {
                triangles,
                vein_lines,
                center_circle,
                center: placement.position,
                phase: placement.phase,
            });
        }

        let mut flowers = Vec::new();
        for placement in flower_placements.iter() {
            let palette = &FLOWER_PALETTES[placement.palette % FLOWER_PALETTES.len()];
            let radius = placement.radius * FLOWER_RADIUS_SCALE;
            let mut triangles = Vec::new();

            let mut ring = |count: usize,
                            length: f32,
                            width: f32,
                            angle_offset: f32,
                            primary: [f32; 3],
                            alternate: [f32; 3]| {
                for index in 0..count {
                    let angle = placement.rotation + angle_offset
                        + index as f32 / count as f32 * TAU;
                    let direction = vec2(angle.cos(), angle.sin());
                    let side = vec2(-direction.y, direction.x);
                    let base = direction.mul(radius * 0.12);
                    let tip = direction.mul(radius * length);
                    let half_width = radius * width;
                    let color = if index % 3 == 0 { alternate } else { primary };
                    for p in [
                        base.add(side.mul(half_width)),
                        tip,
                        base.sub(side.mul(half_width)),
                    ] {
                        triangles.push(Vertex { position: [p.x, p.y], color });
                    }
                }
            };

            ring(8, 1.0, 0.22, 0.0, palette.outer_petal, palette.petal_light);
            ring(
                6,
                0.66,
                0.19,
                std::f32::consts::PI / 6.0,
                palette.inner_petal,
                palette.petal_light,
            );

            for (circle_radius, color) in [
                (radius * 0.28, palette.center_dark),
                (radius * 0.18, palette.center),
            ] {
                for index in 0..8 {
                    let angle_a = index as f32 / 8.0 * TAU;
                    let angle_b = (index + 1) as f32 / 8.0 * TAU;
                    for p in [
                        Vec2::default(),
                        vec2(angle_a.cos() * circle_radius, angle_a.sin() * circle_radius),
                        vec2(angle_b.cos() * circle_radius, angle_b.sin() * circle_radius),
                    ] {
                        triangles.push(Vertex { position: [p.x, p.y], color });
                    }
                }
            }

            flowers.push(LocalFlower { triangles });
        }

        Self {
            placements,
            flower_placements,
            leaves,
            flowers,
            visible_leaves: VISIBLE_LEAF_COUNT,
            visible_flowers: VISIBLE_FLOWER_COUNT,
        }
    }

    pub fn visible_leaves(&self) -> usize {
        self.visible_leaves
    }

    pub fn visible_flowers(&self) -> usize {
        self.visible_flowers
    }

    pub fn max_leaves(&self) -> usize {
        self.leaves.len()
    }

    pub fn max_flowers(&self) -> usize {
        self.flowers.len()
    }

    pub fn set_visible_leaves(&mut self, count: usize) -> usize {
        self.visible_leaves = count.min(self.leaves.len());
        self.visible_leaves
    }

    pub fn set_visible_flowers(&mut self, count: usize) -> usize {
        self.visible_flowers = count.min(self.flowers.len());
        self.visible_flowers
    }

    pub fn update(&self, time: f32, frame: &mut LotusFrame, sprited: [bool; 2]) {
        frame.shadow.clear();
        frame.leaves.clear();
        frame.veins.clear();
        frame.flowers.clear();
        frame.leaf_sprites.clear();

        let mut centers = vec![Vec2::default(); self.placements.len()];
        for (index, leaf) in self.leaves.iter().enumerate() {
            if index >= self.visible_leaves {
                continue;
            }
            let center = vec2(
                leaf.center.x + (time * 0.12 + leaf.phase).sin() * DRIFT_X,
                leaf.center.y + (time * 0.15 + leaf.phase * 1.3).cos() * DRIFT_Y,
            );
            centers[index] = center;
            let sway = (time * 0.085 + leaf.phase).sin() * ROTATION_AMOUNT;
            let pulse = 1.0 + (time * 0.11 + leaf.phase).sin() * 0.012;
            let placement = &self.placements[index];

            if sprited[placement.palette % 2] {
                frame.leaf_sprites.push(LeafSprite {
                    center,
                    rotation: sway,
                    size: placement.radius * RADIUS_SCALE * 2.4 * pulse,
                    palette: placement.palette % 2,
                });
                let shadow_center = center.add(SHADOW_OFFSET.mul(1.02));
                let radius = placement.radius * RADIUS_SCALE;
                push_circle(&mut frame.shadow, shadow_center, radius, 14);
                continue;
            }

            let transform = |local: Vec2, scale: f32, offset: Vec2| {
                let rotated = vec2(
                    local.x * sway.cos() - local.y * sway.sin(),
                    local.x * sway.sin() + local.y * sway.cos(),
                );
                center.add(rotated.mul(scale)).add(offset)
            };

            for vertex in &leaf.triangles {
                let p = transform(vec2(vertex.position[0], vertex.position[1]), pulse, Vec2::default());
                frame.leaves.push(Vertex { position: [p.x, p.y], color: vertex.color });
                let s = transform(
                    vec2(vertex.position[0], vertex.position[1]),
                    pulse * 1.02,
                    SHADOW_OFFSET,
                );
                frame.shadow.push(Vertex { position: [s.x, s.y], color: [1.0, 1.0, 1.0] });
            }
            for vertex in &leaf.center_circle {
                let p = transform(vec2(vertex.position[0], vertex.position[1]), pulse, Vec2::default());
                frame.leaves.push(Vertex { position: [p.x, p.y], color: vertex.color });
            }
            for vertex in &leaf.vein_lines {
                let p = transform(vec2(vertex.position[0], vertex.position[1]), pulse, Vec2::default());
                frame.veins.push(Vertex { position: [p.x, p.y], color: vertex.color });
            }
        }

        for (index, flower) in self.flowers.iter().enumerate() {
            if index >= self.visible_flowers {
                continue;
            }
            let placement = &self.flower_placements[index];
            if placement.leaf_index >= self.visible_leaves {
                continue;
            }
            let Some(leaf_center) = centers.get(placement.leaf_index) else {
                continue;
            };
            let rotation = (time * 0.12 + self.placements[placement.leaf_index].phase).sin() * 0.04;
            let origin = leaf_center.add(placement.offset);
            for vertex in &flower.triangles {
                let local = vec2(vertex.position[0], vertex.position[1]);
                let rotated = vec2(
                    local.x * rotation.cos() - local.y * rotation.sin(),
                    local.x * rotation.sin() + local.y * rotation.cos(),
                );
                let p = origin.add(rotated);
                frame.flowers.push(Vertex { position: [p.x, p.y], color: vertex.color });
            }
        }
    }
}

fn push_circle(out: &mut Vec<Vertex>, center: Vec2, radius: f32, segments: usize) {
    for index in 0..segments {
        let angle_a = index as f32 / segments as f32 * TAU;
        let angle_b = (index + 1) as f32 / segments as f32 * TAU;
        for point in [
            center,
            center.add(vec2(angle_a.cos() * radius, angle_a.sin() * radius)),
            center.add(vec2(angle_b.cos() * radius, angle_b.sin() * radius)),
        ] {
            out.push(Vertex {
                position: [point.x, point.y],
                color: [1.0, 1.0, 1.0],
            });
        }
    }
}
