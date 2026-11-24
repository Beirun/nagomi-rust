//! Port of duckweed.ts + duckweed-geometry.ts. Transforms run on the CPU.

use crate::fish::Vertex;
use crate::sim::{vec2, Vec2, XorShift32};

const TAU: f32 = std::f32::consts::TAU;
const LEAF_SEGMENTS: usize = 7;
const HIGHLIGHT_SEGMENTS: usize = 5;
const HIGHLIGHT_MIN_RADIUS: f32 = 1.55;

const VISIBLE_PATCH_COUNT: usize = 8;
const MAX_PATCHES: usize = 16;
const MIN_LEAF_RADIUS: f32 = 1.05;
const MAX_LEAF_RADIUS: f32 = 3.35;
const VERTICAL_SCALE: f32 = 0.76;
const PAIR_CHANCE: f32 = 0.42;
const SPREAD_EXPONENT: f32 = 0.68;
const DRIFT_X: f32 = 5.55;
const DRIFT_Y: f32 = 5.42;
const ROTATION_AMOUNT: f32 = 0.045;
const SHADOW_OFFSET: Vec2 = vec2(1.4, 5.1);

pub const SHADOW_COLOR: [f32; 3] = [0x12 as f32 / 255.0, 0x3b as f32 / 255.0, 0x2d as f32 / 255.0];
pub const SHADOW_OPACITY: f32 = 0.24;

const fn rgb(hex: u32) -> [f32; 3] {
    [
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
    ]
}

struct Palette {
    base: [f32; 3],
    light: [f32; 3],
    shade: [f32; 3],
    center: [f32; 3],
}

const PALETTES: [Palette; 2] = [
    Palette { base: rgb(0x6fc94f), light: rgb(0x9be66c), shade: rgb(0x45963e), center: rgb(0xc3ee75) },
    Palette { base: rgb(0x83d35b), light: rgb(0xb1ed76), shade: rgb(0x549e43), center: rgb(0xd0f28a) },
];

#[derive(Clone, Copy)]
struct Patch {
    position: Vec2,
    radius: f32,
    count: usize,
    phase: f32,
    palette: usize,
}

const PATCHES: [Patch; 8] = [
    Patch { position: vec2(28.0, 45.0), radius: 30.0, count: 42, phase: 0.3, palette: 0 },
    Patch { position: vec2(102.0, 17.0), radius: 22.0, count: 28, phase: 1.7, palette: 1 },
    Patch { position: vec2(447.0, 34.0), radius: 77.0, count: 138, phase: 2.8, palette: 0 },
    Patch { position: vec2(470.0, 116.0), radius: 25.0, count: 34, phase: 4.1, palette: 1 },
    Patch { position: vec2(451.0, 225.0), radius: 31.0, count: 44, phase: 5.3, palette: 0 },
    Patch { position: vec2(378.0, 259.0), radius: 22.0, count: 29, phase: 0.9, palette: 1 },
    Patch { position: vec2(71.0, 244.0), radius: 29.0, count: 40, phase: 3.4, palette: 0 },
    Patch { position: vec2(13.0, 168.0), radius: 64.0, count: 200, phase: 4.8, palette: 0 },
];

fn random_unit(seed: f32) -> f32 {
    let value = (seed * 12.9898 + 78.233).sin() * 43758.5453;
    value - value.floor()
}

struct LeafShape {
    offset: Vec2,
    radius: f32,
    angle: f32,
    tone: f32,
    paired: bool,
}

pub struct DuckweedFrame {
    pub shadow: Vec<Vertex>,
    pub leaves: Vec<Vertex>,
    pub highlights: Vec<Vertex>,
}

impl DuckweedFrame {
    pub fn new() -> Self {
        Self {
            shadow: Vec::new(),
            leaves: Vec::new(),
            highlights: Vec::new(),
        }
    }
}

pub struct Duckweed {
    patches: Vec<Patch>,
    leaves: Vec<Vec<LeafShape>>,
    visible_patches: usize,
}

impl Duckweed {
    pub fn new() -> Self {
        let mut rng = XorShift32::new(0x0d0c_a11e);
        let mut patches: Vec<Patch> = PATCHES.to_vec();
        while patches.len() < MAX_PATCHES {
            let position = match (rng.unit() * 4.0) as u32 {
                0 => vec2(
                    16.0 + rng.unit() * 449.0,
                    8.0 + rng.unit() * 35.0,
                ),
                1 => vec2(
                    438.0 + rng.unit() * 35.0,
                    16.0 + rng.unit() * 239.0,
                ),
                2 => vec2(
                    16.0 + rng.unit() * 449.0,
                    228.0 + rng.unit() * 35.0,
                ),
                _ => vec2(
                    8.0 + rng.unit() * 35.0,
                    16.0 + rng.unit() * 239.0,
                ),
            };
            patches.push(Patch {
                position,
                radius: 18.0 + rng.unit() * 25.0,
                count: 18 + (rng.unit() * 15.0) as usize,
                phase: rng.unit() * TAU,
                palette: if rng.unit() < 0.5 { 0 } else { 1 },
            });
        }

        let mut leaves = Vec::new();
        for (patch_index, patch) in patches.iter().enumerate() {
            let mut shapes = Vec::with_capacity(patch.count);
            for index in 0..patch.count {
                let seed = (patch_index * 1013 + index * 37 + 11) as f32;
                let radius_amount = random_unit(seed + 1.0).powf(SPREAD_EXPONENT);
                let distance = patch.radius * radius_amount;
                let angle = random_unit(seed + 2.0) * TAU;
                shapes.push(LeafShape {
                    offset: vec2(
                        angle.cos() * distance,
                        angle.sin() * distance * 0.74,
                    ),
                    radius: MIN_LEAF_RADIUS
                        + random_unit(seed + 3.0) * (MAX_LEAF_RADIUS - MIN_LEAF_RADIUS),
                    angle: random_unit(seed + 4.0) * TAU,
                    tone: random_unit(seed + 6.0),
                    paired: random_unit(seed + 7.0) < PAIR_CHANCE,
                });
            }
            leaves.push(shapes);
        }
        Self {
            patches,
            leaves,
            visible_patches: VISIBLE_PATCH_COUNT,
        }
    }

    pub fn visible_patches(&self) -> usize {
        self.visible_patches
    }

    pub fn max_patches(&self) -> usize {
        self.leaves.len()
    }

    pub fn set_visible_patches(&mut self, count: usize) -> usize {
        self.visible_patches = count.min(self.leaves.len());
        self.visible_patches
    }

    pub fn update(&self, time: f32, frame: &mut DuckweedFrame) {
        frame.shadow.clear();
        frame.leaves.clear();
        frame.highlights.clear();

        for (patch_index, patch) in self
            .patches
            .iter()
            .enumerate()
            .take(self.visible_patches)
        {
            let palette = &PALETTES[patch.palette % PALETTES.len()];
            let center = vec2(
                patch.position.x + (time * 0.1 + patch.phase).sin() * DRIFT_X,
                patch.position.y + (time * 0.13 + patch.phase * 1.4).cos() * DRIFT_Y,
            );
            let rotation = (time * 0.075 + patch.phase).sin() * ROTATION_AMOUNT;
            let (sin, cos) = rotation.sin_cos();

            let place = |local: Vec2| -> Vec2 {
                vec2(local.x * cos - local.y * sin, local.x * sin + local.y * cos).add(center)
            };

            for leaf in &self.leaves[patch_index] {
                let color = if leaf.tone < 0.24 {
                    palette.light
                } else if leaf.tone > 0.82 {
                    palette.shade
                } else {
                    palette.base
                };

                push_leaf(&mut frame.leaves, &place, leaf.offset, leaf.radius, leaf.angle, color);
                push_leaf(
                    &mut frame.shadow,
                    &|local| place(local).add(SHADOW_OFFSET),
                    leaf.offset,
                    leaf.radius,
                    leaf.angle,
                    [1.0, 1.0, 1.0],
                );

                if leaf.radius > HIGHLIGHT_MIN_RADIUS {
                    let highlight_center = leaf.offset.sub(vec2(
                        leaf.angle.cos() * leaf.radius * 0.18,
                        leaf.angle.sin() * leaf.radius * 0.18,
                    ));
                    let highlight_radius = (leaf.radius * 0.14).max(0.22);
                    for index in 0..HIGHLIGHT_SEGMENTS {
                        let angle_a = index as f32 / HIGHLIGHT_SEGMENTS as f32 * TAU;
                        let angle_b = (index + 1) as f32 / HIGHLIGHT_SEGMENTS as f32 * TAU;
                        let a = place(highlight_center.add(vec2(
                            angle_a.cos() * highlight_radius,
                            angle_a.sin() * highlight_radius,
                        )));
                        let b = place(highlight_center.add(vec2(
                            angle_b.cos() * highlight_radius,
                            angle_b.sin() * highlight_radius,
                        )));
                        let c = place(highlight_center);
                        for p in [c, a, b] {
                            frame.highlights.push(Vertex {
                                position: [p.x, p.y],
                                color: palette.center,
                            });
                        }
                    }
                }

                if leaf.paired {
                    let pair_center = leaf.offset.add(vec2(
                        (leaf.angle + 0.8).cos() * leaf.radius * 0.92,
                        (leaf.angle + 0.8).sin() * leaf.radius * 0.92,
                    ));
                    push_leaf(
                        &mut frame.leaves,
                        &place,
                        pair_center,
                        leaf.radius * 0.72,
                        leaf.angle + 1.15,
                        palette.light,
                    );
                    push_leaf(
                        &mut frame.shadow,
                        &|local| place(local).add(SHADOW_OFFSET),
                        pair_center,
                        leaf.radius * 0.72,
                        leaf.angle + 1.15,
                        [1.0, 1.0, 1.0],
                    );
                }
            }
        }
    }
}

fn push_leaf(
    out: &mut Vec<Vertex>,
    place: &impl Fn(Vec2) -> Vec2,
    center: Vec2,
    radius: f32,
    angle: f32,
    color: [f32; 3],
) {
    let (sin, cos) = angle.sin_cos();
    for index in 0..LEAF_SEGMENTS {
        let angle_a = index as f32 / LEAF_SEGMENTS as f32 * TAU;
        let angle_b = (index + 1) as f32 / LEAF_SEGMENTS as f32 * TAU;
        let ellipse_point = |a: f32| {
            let local_x = a.cos() * radius;
            let local_y = a.sin() * radius * VERTICAL_SCALE;
            center.add(vec2(
                local_x * cos - local_y * sin,
                local_x * sin + local_y * cos,
            ))
        };
        let a = place(ellipse_point(angle_a));
        let b = place(ellipse_point(angle_b));
        let c = place(center);
        for p in [c, a, b] {
            out.push(Vertex {
                position: [p.x, p.y],
                color,
            });
        }
    }
}
