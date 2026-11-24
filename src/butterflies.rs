//! Port of butterflies.ts: wander-flight behavior plus the wing/body drawing.

use crate::fish::Vertex;
use crate::sim::{clamp, vec2, wrap_angle, Vec2, POND_H, POND_W};

const TAU: f32 = std::f32::consts::TAU;
const VISIBLE_COUNT: usize = 4;
const EDGE_MARGIN: f32 = 14.0;
const BODY_LENGTH: f32 = 3.8;
const BODY_WIDTH: f32 = 0.32;
const HEAD_RADIUS: f32 = 0.72;
const WING_LENGTH: f32 = 4.7;
const WING_WIDTH: f32 = 5.4;
const WING_SPOT_RADIUS: f32 = 0.58;
const MIN_SPEED: f32 = 8.5;
const MAX_SPEED: f32 = 30.5;
const TURN_SMOOTHING: f32 = 3.1;
const TURN_INTERVAL: (f32, f32) = (0.32, 1.15);
const TURN_ANGLE: f32 = 0.72;
const SHARP_TURN_CHANCE: f32 = 0.18;
const SHARP_TURN_ANGLE: f32 = 1.45;
const CURVED_FLIGHT_STRENGTH: f32 = 0.34;
const CURVED_FLIGHT_FREQUENCY: (f32, f32) = (0.65, 1.35);
const SPEED_VARIATION: f32 = 0.27;
const FLAP_SPEED: (f32, f32) = (17.5, 31.5);

pub const SHADOW_COLOR: [f32; 3] = [0x17 as f32 / 255.0, 0x37 as f32 / 255.0, 0x2f as f32 / 255.0];
pub const SHADOW_OPACITY: f32 = 0.2;
const SHADOW_OFFSET: Vec2 = vec2(2.4, 3.2);
const SHADOW_SCALE: f32 = 0.82;

const fn rgb(hex: u32) -> [f32; 3] {
    [
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
    ]
}

struct Palette {
    wing: [f32; 3],
    wing_light: [f32; 3],
    accent: [f32; 3],
    body: [f32; 3],
}

const PALETTES: [Palette; 4] = [
    Palette { wing: rgb(0xf3a64c), wing_light: rgb(0xffd36b), accent: rgb(0x75448b), body: rgb(0x3e2d35) },
    Palette { wing: rgb(0x71bce8), wing_light: rgb(0xb8e4f5), accent: rgb(0x315b9d), body: rgb(0x293747) },
    Palette { wing: rgb(0xe9789d), wing_light: rgb(0xffb4c5), accent: rgb(0x8f416b), body: rgb(0x49303c) },
    Palette { wing: rgb(0xc4df58), wing_light: rgb(0xeaf68a), accent: rgb(0x508c61), body: rgb(0x334239) },
];

const SPAWNS: [(f32, f32); 4] = [(56.0, 61.0), (416.0, 71.0), (394.0, 214.0), (101.0, 218.0)];

#[derive(Clone, Copy)]
struct Butterfly {
    position: Vec2,
    heading: f32,
    speed: f32,
    cruise_speed: f32,
    turn_target: f32,
    turn_timer: f32,
    flap_speed: f32,
    flap_phase: f32,
    curve_frequency: f32,
    speed_phase: f32,
    phase: f32,
    palette: usize,
    random_state: u32,
}

impl Butterfly {
    fn random(&mut self) -> f32 {
        self.random_state = self
            .random_state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.random_state as f32 / 4_294_967_296.0
    }

    fn range(&mut self, range: (f32, f32)) -> f32 {
        range.0 + self.random() * (range.1 - range.0)
    }
}

pub struct Butterflies {
    butterflies: Vec<Butterfly>,
    visible: usize,
}

pub struct ButterflyFrame {
    pub shadow: Vec<Vertex>,
    pub shapes: Vec<Vertex>,
}

impl ButterflyFrame {
    pub fn new() -> Self {
        Self {
            shadow: Vec::new(),
            shapes: Vec::new(),
        }
    }
}

impl Butterflies {
    pub fn new() -> Self {
        let mut butterflies = Vec::new();
        for (index, spawn) in SPAWNS.iter().enumerate() {
            let mut butterfly = Butterfly {
                position: vec2(spawn.0, spawn.1),
                heading: index as f32 * 1.7,
                speed: 14.0,
                cruise_speed: 0.0,
                turn_target: 0.0,
                turn_timer: 0.0,
                flap_speed: 20.0,
                flap_phase: 0.0,
                curve_frequency: 1.0,
                speed_phase: 0.0,
                phase: index as f32 * 1.3,
                palette: index % PALETTES.len(),
                random_state: 0x9e37_79b9 ^ (index as u32 + 1).wrapping_mul(0x85eb_ca6b),
            };
            butterfly.cruise_speed = butterfly.range((MIN_SPEED, MAX_SPEED));
            butterfly.flap_speed = butterfly.range(FLAP_SPEED);
            butterfly.curve_frequency = butterfly.range(CURVED_FLIGHT_FREQUENCY);
            butterfly.turn_target = butterfly.range((-TURN_ANGLE, TURN_ANGLE));
            butterflies.push(butterfly);
        }
        Self {
            butterflies,
            visible: VISIBLE_COUNT,
        }
    }

    pub fn visible(&self) -> usize {
        self.visible
    }

    pub fn max_butterflies(&self) -> usize {
        self.butterflies.len()
    }

    pub fn set_visible(&mut self, count: usize) -> usize {
        self.visible = count.min(self.butterflies.len());
        self.visible
    }

    pub fn update(&mut self, dt: f32, time: f32) {
        for index in 0..self.butterflies.len().min(self.visible) {
            let mut b = self.butterflies[index];

            b.turn_timer -= dt;
            if b.turn_timer <= 0.0 {
                b.turn_timer = b.range(TURN_INTERVAL);
                let sharp = b.random() < SHARP_TURN_CHANCE;
                let maximum = if sharp { SHARP_TURN_ANGLE } else { TURN_ANGLE };
                b.turn_target = (b.random() * 2.0 - 1.0) * maximum;
            }

            let curve = (time * b.curve_frequency + b.phase).sin()
                * CURVED_FLIGHT_STRENGTH;
            let desired_turn = b.turn_target + curve;
            b.heading = wrap_angle(b.heading + desired_turn * TURN_SMOOTHING * dt);

            // Steer back inside the pond near the edges.
            let margin = EDGE_MARGIN;
            let mut toward = Vec2::default();
            if b.position.x < margin {
                toward.x += (margin - b.position.x) / margin;
            }
            if b.position.x > POND_W - margin {
                toward.x -= (b.position.x - (POND_W - margin)) / margin;
            }
            if b.position.y < margin {
                toward.y += (margin - b.position.y) / margin;
            }
            if b.position.y > POND_H - margin {
                toward.y -= (b.position.y - (POND_H - margin)) / margin;
            }
            if toward.length() > 0.01 {
                let target_heading = toward.y.atan2(toward.x);
                let error = wrap_angle(target_heading - b.heading);
                b.heading = wrap_angle(b.heading + clamp(error, -2.0 * dt, 2.0 * dt) * 3.0);
            }

            let speed = b.cruise_speed
                * (1.0 + (time * 0.7 + b.speed_phase).sin() * SPEED_VARIATION);
            b.speed = speed;
            b.position = b
                .position
                .add(vec2(b.heading.cos(), b.heading.sin()).mul(speed * dt));

            b.flap_phase += b.flap_speed * dt;

            self.butterflies[index] = b;
        }
    }

    pub fn draw(&self, time: f32, frame: &mut ButterflyFrame) {
        frame.shadow.clear();
        frame.shapes.clear();

        for butterfly in self.butterflies.iter().take(self.visible) {
            let palette = &PALETTES[butterfly.palette % PALETTES.len()];
            let forward = vec2(butterfly.heading.cos(), butterfly.heading.sin());
            let side = forward.perpendicular();
            let flap = 0.3
                + (time * butterfly.flap_speed + butterfly.phase).sin().abs() * 0.7;
            let wing_width = WING_WIDTH * (0.28 + flap * 0.72);

            draw_wings(
                &mut frame.shadow,
                butterfly.position.add(SHADOW_OFFSET),
                forward,
                side,
                WING_LENGTH * SHADOW_SCALE,
                wing_width * SHADOW_SCALE,
                SHADOW_COLOR,
                SHADOW_COLOR,
            );
            draw_wings(
                &mut frame.shapes,
                butterfly.position,
                forward,
                side,
                WING_LENGTH,
                wing_width,
                palette.wing,
                palette.wing_light,
            );

            let body_front = butterfly.position.add(forward.mul(BODY_LENGTH * 0.58));
            let body_back = butterfly.position.sub(forward.mul(BODY_LENGTH * 0.58));
            let body_side = side.mul(BODY_WIDTH);
            push_triangle(
                &mut frame.shapes,
                body_front.add(body_side),
                body_front.sub(body_side),
                body_back.sub(body_side),
                palette.body,
            );
            push_triangle(
                &mut frame.shapes,
                body_front.add(body_side),
                body_back.sub(body_side),
                body_back.add(body_side),
                palette.body,
            );
            push_circle(&mut frame.shapes, body_front, HEAD_RADIUS, palette.body, 6);

            let spot_distance = wing_width * 0.68;
            for direction in [-1.0f32, 1.0] {
                let center = butterfly
                    .position
                    .add(side.mul(spot_distance * direction))
                    .sub(forward.mul(WING_LENGTH * 0.08));
                push_circle(
                    &mut frame.shapes,
                    center,
                    WING_SPOT_RADIUS,
                    palette.accent,
                    5,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_wings(
    out: &mut Vec<Vertex>,
    center: Vec2,
    forward: Vec2,
    side: Vec2,
    length: f32,
    width: f32,
    primary: [f32; 3],
    light: [f32; 3],
) {
    for direction in [-1.0f32, 1.0] {
        let shoulder = center.add(forward.mul(length * 0.2));
        let outer_front = center
            .add(side.mul(width * direction))
            .add(forward.mul(length * 0.42));
        let outer_back = center
            .add(side.mul(width * 0.82 * direction))
            .sub(forward.mul(length * 0.48));
        let tail = center.sub(forward.mul(length * 0.68));
        push_triangle(out, shoulder, outer_front, outer_back, light);
        push_triangle(out, shoulder, outer_back, tail, primary);
    }
}

fn push_triangle(out: &mut Vec<Vertex>, a: Vec2, b: Vec2, c: Vec2, color: [f32; 3]) {
    for p in [a, b, c] {
        out.push(Vertex { position: [p.x, p.y], color });
    }
}

fn push_circle(
    out: &mut Vec<Vertex>,
    center: Vec2,
    radius: f32,
    color: [f32; 3],
    segments: usize,
) {
    for index in 0..segments {
        let angle_a = index as f32 / segments as f32 * TAU;
        let angle_b = (index + 1) as f32 / segments as f32 * TAU;
        push_triangle(
            out,
            center,
            center.add(vec2(angle_a.cos() * radius, angle_a.sin() * radius)),
            center.add(vec2(angle_b.cos() * radius, angle_b.sin() * radius)),
            color,
        );
    }
}
