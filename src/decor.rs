//! Pond decorations that sit on the water: shoreline rocks, and the fallen
//! leaves that drift across the maple pond.

use crate::duckweed::DuckweedFrame;
use crate::fish::Vertex;
use crate::lotus::LotusFrame;
use crate::pond::Pond;
use crate::sim::{vec2, Vec2, XorShift32};

const ROCK_LIGHT: [f32; 3] = [154.0 / 255.0, 150.0 / 255.0, 138.0 / 255.0];
const ROCK_BASE: [f32; 3] = [116.0 / 255.0, 112.0 / 255.0, 102.0 / 255.0];
const ROCK_DARK: [f32; 3] = [79.0 / 255.0, 77.0 / 255.0, 70.0 / 255.0];

const LEAF_COLORS: [[f32; 3]; 5] = [
    [206.0 / 255.0, 69.0 / 255.0, 44.0 / 255.0],
    [224.0 / 255.0, 123.0 / 255.0, 40.0 / 255.0],
    [168.0 / 255.0, 43.0 / 255.0, 30.0 / 255.0],
    [229.0 / 255.0, 161.0 / 255.0, 58.0 / 255.0],
    [190.0 / 255.0, 84.0 / 255.0, 32.0 / 255.0],
];

const CLUSTERS: [[f32; 2]; 10] = [
    [22.0, 44.0],
    [50.0, 74.0],
    [410.0, 26.0],
    [452.0, 58.0],
    [470.0, 140.0],
    [58.0, 236.0],
    [96.0, 252.0],
    [390.0, 244.0],
    [436.0, 208.0],
    [250.0, 18.0],
];

const ROCK_POINTS: usize = 9;
const MAPLE_LEAF_COUNT: usize = 210;
const MAPLE_LOBES: usize = 5;

struct Rock {
    center: Vec2,
    radius: f32,
    shape: [f32; ROCK_POINTS],
}

struct MapleLeaf {
    home: Vec2,
    radius: f32,
    rotation: f32,
    phase: f32,
    color: [f32; 3],
}

pub struct Decor {
    rocks: Vec<Rock>,
    leaves: Vec<MapleLeaf>,
}

impl Decor {
    pub fn new() -> Self {
        let mut rng = XorShift32::new(0x70c0_11e5);
        let mut rocks = Vec::new();
        for [x, y] in CLUSTERS {
            let count = 1 + (rng.unit() * 3.0) as usize;
            for _ in 0..count {
                let offset = vec2(rng.range(-9.0, 9.0), rng.range(-7.0, 7.0));
                let radius = rng.range(4.5, 12.0);
                let mut shape = [1.0; ROCK_POINTS];
                for value in shape.iter_mut() {
                    *value = rng.range(0.78, 1.16);
                }
                rocks.push(Rock {
                    center: vec2(x, y).add(offset),
                    radius,
                    shape,
                });
            }
        }

        let mut leaves = Vec::new();
        for _ in 0..MAPLE_LEAF_COUNT {
            leaves.push(MapleLeaf {
                home: vec2(rng.range(8.0, 472.0), rng.range(8.0, 262.0)),
                radius: rng.range(2.2, 4.2),
                rotation: rng.range(0.0, std::f32::consts::TAU),
                phase: rng.range(0.0, std::f32::consts::TAU),
                color: LEAF_COLORS[(rng.unit() * LEAF_COLORS.len() as f32) as usize
                    % LEAF_COLORS.len()],
            });
        }

        Self { rocks, leaves }
    }

    pub fn update(
        &self,
        pond: Pond,
        time: f32,
        lotus: &mut LotusFrame,
        duckweed: &mut DuckweedFrame,
    ) {
        if pond.has_rocks() {
            let lift = if matches!(pond, Pond::Lagoon) { 1.16 } else { 1.0 };
            self.draw_rocks(lift, lotus);
        }
        if pond.has_maple_leaves() {
            self.draw_leaves(time, duckweed);
        }
    }

    fn draw_rocks(&self, lift: f32, frame: &mut LotusFrame) {
        let tint = |color: [f32; 3]| {
            [
                (color[0] * lift).min(1.0),
                (color[1] * lift).min(1.0),
                (color[2] * lift).min(1.0),
            ]
        };
        for rock in &self.rocks {
            let mut rim = Vec::with_capacity(ROCK_POINTS);
            for (index, wobble) in rock.shape.iter().enumerate() {
                let angle = index as f32 / ROCK_POINTS as f32 * std::f32::consts::TAU;
                let radius = rock.radius * wobble;
                rim.push(vec2(angle.cos() * radius, angle.sin() * radius * 0.82));
            }
            for index in 0..ROCK_POINTS {
                let a = rim[index];
                let b = rim[(index + 1) % ROCK_POINTS];
                let light = index < ROCK_POINTS / 2;
                let color = if light {
                    tint(ROCK_LIGHT)
                } else {
                    tint(ROCK_BASE)
                };
                frame.leaves.push(Vertex {
                    position: [rock.center.x, rock.center.y],
                    color: tint(ROCK_DARK),
                });
                frame.leaves.push(Vertex {
                    position: [rock.center.x + a.x, rock.center.y + a.y],
                    color,
                });
                frame.leaves.push(Vertex {
                    position: [rock.center.x + b.x, rock.center.y + b.y],
                    color,
                });
            }
            let shadow_center = rock.center.add(vec2(3.0, 6.5));
            for index in 0..ROCK_POINTS {
                let a = rim[index].mul(1.06);
                let b = rim[(index + 1) % ROCK_POINTS].mul(1.06);
                for p in [
                    Vec2::default(),
                    a,
                    b,
                ] {
                    frame.shadow.push(Vertex {
                        position: [shadow_center.x + p.x, shadow_center.y + p.y],
                        color: [1.0, 1.0, 1.0],
                    });
                }
            }
        }
    }

    fn draw_leaves(&self, time: f32, frame: &mut DuckweedFrame) {
        for leaf in &self.leaves {
            let position = vec2(
                leaf.home.x + (time * 0.05 + leaf.phase).sin() * 3.0,
                leaf.home.y + (time * 0.045 + leaf.phase * 1.3).cos() * 2.5,
            );
            let rotation = leaf.rotation + (time * 0.1 + leaf.phase).sin() * 0.4;
            let (sin, cos) = rotation.sin_cos();
            let mut points = Vec::with_capacity(MAPLE_LOBES * 2);
            for index in 0..MAPLE_LOBES * 2 {
                let angle = index as f32 / (MAPLE_LOBES * 2) as f32 * std::f32::consts::TAU;
                let radius = if index % 2 == 0 {
                    leaf.radius
                } else {
                    leaf.radius * 0.42
                };
                let local = vec2(angle.cos() * radius, angle.sin() * radius * 0.94);
                points.push(vec2(
                    local.x * cos - local.y * sin,
                    local.x * sin + local.y * cos,
                ));
            }
            for index in 0..points.len() {
                let a = points[index];
                let b = points[(index + 1) % points.len()];
                frame.leaves.push(Vertex {
                    position: [position.x, position.y],
                    color: leaf.color,
                });
                frame.leaves.push(Vertex {
                    position: [position.x + a.x, position.y + a.y],
                    color: leaf.color,
                });
                frame.leaves.push(Vertex {
                    position: [position.x + b.x, position.y + b.y],
                    color: leaf.color,
                });
            }
            let shadow = position.add(vec2(1.2, 2.6));
            for index in 0..points.len() {
                let a = points[index];
                let b = points[(index + 1) % points.len()];
                for p in [Vec2::default(), a, b] {
                    frame.shadow.push(Vertex {
                        position: [shadow.x + p.x, shadow.y + p.y],
                        color: [1.0, 1.0, 1.0],
                    });
                }
            }
        }
    }
}
