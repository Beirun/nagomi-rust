//! Port of tiny-fish.ts + tiny-fish-renderer.ts.

use crate::fish::Vertex;
use crate::sim::{clamp, from_angle, vec2, wrap_angle, Vec2, XorShift32, POND_H, POND_W};

const TAU: f32 = std::f32::consts::TAU;

const VISIBLE_SCHOOL_COUNT: usize = 3;
const BODY_LENGTH: (f32, f32) = (5.8, 8.2);
const BODY_WIDTH_RATIO: (f32, f32) = (0.1, 0.32);
const TAIL_LENGTH_SCALE: f32 = 0.34;
const TAIL_WIDTH_SCALE: f32 = 0.92;
const FIN_REACH_SCALE: f32 = 1.28;
const EYE_RADIUS: f32 = 0.28;
const CRUISE_SPEED: (f32, f32) = (19.0, 27.0);
const SPEED_VARIATION: f32 = 0.46;
const EDGE_MARGIN: f32 = 14.0;
const NEIGHBOUR_RADIUS: f32 = 25.0;
const SEPARATION_RADIUS: f32 = 6.2;
const COHESION_STRENGTH: f32 = 0.62;
const ALIGNMENT_STRENGTH: f32 = 0.56;
const SEPARATION_STRENGTH: f32 = 2.8;
const SWIRL_STRENGTH: f32 = 0.46;
const WANDER_STRENGTH: f32 = 0.34;
const EDGE_STRENGTH: f32 = 4.8;
const STEERING_RESPONSE: f32 = 4.7;
const MAXIMUM_TURN_RATE: f32 = 2.4;
const FLEE_REACTION_RADIUS: f32 = 270.0;
const FLEE_PROPAGATION_SPEED: f32 = 180.0;
const FLEE_RANDOM_DELAY: f32 = 0.16;
const FLEE_DURATION: (f32, f32) = (1.45, 2.35);
const FLEE_SPEED: (f32, f32) = (49.0, 64.0);
const FLEE_DIRECTION_STRENGTH: f32 = 5.8;
const FLEE_SCHOOLING_STRENGTH: f32 = 0.58;
const FLEE_INITIAL_IMPULSE: f32 = 13.0;

const SHADOW_COLOR: [f32; 3] = [0x12 as f32 / 255.0, 0x35 as f32 / 255.0, 0x2f as f32 / 255.0];
pub const SHADOW_OPACITY: f32 = 0.28;
const SHADOW_OFFSET: Vec2 = vec2(1.5, 2.8);

const fn rgb(hex: u32) -> [f32; 3] {
    [
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
    ]
}

struct Palette {
    body: [f32; 3],
    light: [f32; 3],
    accent: [f32; 3],
    fin: [f32; 3],
    eye: [f32; 3],
}

const PALETTES: [Palette; 4] = [
    Palette { body: rgb(0xffe66d), light: rgb(0xfff3a0), accent: rgb(0xff8c42), fin: rgb(0xffc857), eye: rgb(0x203638) },
    Palette { body: rgb(0x56dffc), light: rgb(0xb2f2ff), accent: rgb(0x3877ed), fin: rgb(0x85edff), eye: rgb(0x173b52) },
    Palette { body: rgb(0xff72ad), light: rgb(0xffbad2), accent: rgb(0xffd05e), fin: rgb(0xff9bc2), eye: rgb(0x4d2940) },
    Palette { body: rgb(0xa8ed48), light: rgb(0xddff8c), accent: rgb(0x38bb78), fin: rgb(0xc5f56d), eye: rgb(0x254535) },
];

struct SchoolSetting {
    x: f32,
    y: f32,
    count: usize,
    heading: f32,
    spread_x: f32,
    spread_y: f32,
    palette: usize,
    size_scale: f32,
    speed_scale: f32,
    swirl_direction: f32,
}

const SCHOOLS: [SchoolSetting; 4] = [
    SchoolSetting { x: 174.0, y: 82.0, count: 24, heading: 0.35, spread_x: 32.0, spread_y: 15.0, palette: 0, size_scale: 0.88, speed_scale: 1.04, swirl_direction: 1.0 },
    SchoolSetting { x: 343.0, y: 174.0, count: 15, heading: 2.75, spread_x: 22.0, spread_y: 11.0, palette: 1, size_scale: 1.08, speed_scale: 0.95, swirl_direction: -1.0 },
    SchoolSetting { x: 139.0, y: 204.0, count: 34, heading: -0.72, spread_x: 40.0, spread_y: 18.0, palette: 2, size_scale: 0.82, speed_scale: 1.12, swirl_direction: 1.0 },
    SchoolSetting { x: 377.0, y: 69.0, count: 19, heading: 2.2, spread_x: 28.0, spread_y: 13.0, palette: 3, size_scale: 0.94, speed_scale: 1.0, swirl_direction: -1.0 },
];

#[derive(Clone, Copy)]
pub struct TinyFish {
    pub position: Vec2,
    pub velocity: Vec2,
    pub body_length: f32,
    pub body_width: f32,
    pub tail_phase: f32,
    pub phase: f32,
    pub palette: usize,
    pub school_index: usize,
    pub cruise_speed: f32,
    pub flee_delay: f32,
    pub flee_time: f32,
    pub flee_duration: f32,
}

struct SchoolRange {
    start: usize,
    count: usize,
    phase: f32,
    setting: usize,
}

pub struct TinySchools {
    pub fish: Vec<TinyFish>,
    ranges: Vec<SchoolRange>,
    random: XorShift32,
    call_point: Vec2,
}

impl TinySchools {
    pub fn new() -> Self {
        let mut schools = Self {
            fish: Vec::new(),
            ranges: Vec::new(),
            random: XorShift32::new(0x51a7_f15c),
            call_point: Vec2::default(),
        };
        schools.reset();
        schools
    }

    fn reset(&mut self) {
        self.fish.clear();
        self.ranges.clear();
        self.random = XorShift32::new(0x51a7_f15c);

        for (school_index, setting) in SCHOOLS.iter().enumerate() {
            let start = self.fish.len();
            for _ in 0..setting.count {
                let angle = self.random.range(0.0, TAU);
                let radius = self.random.unit().sqrt();
                let position = vec2(
                    setting.x + angle.cos() * setting.spread_x * radius,
                    setting.y + angle.sin() * setting.spread_y * radius,
                );
                let heading = setting.heading + self.random.range(-0.34, 0.34);
                let body_length = self.random.range(BODY_LENGTH.0, BODY_LENGTH.1)
                    * setting.size_scale;
                let cruise_speed =
                    self.random.range(CRUISE_SPEED.0, CRUISE_SPEED.1) * setting.speed_scale;
                self.fish.push(TinyFish {
                    position,
                    velocity: from_angle(heading).mul(cruise_speed),
                    body_length,
                    body_width: body_length
                        * self.random.range(BODY_WIDTH_RATIO.0, BODY_WIDTH_RATIO.1),
                    tail_phase: self.random.range(0.0, TAU),
                    phase: self.random.range(0.0, TAU),
                    palette: setting.palette,
                    school_index,
                    cruise_speed,
                    flee_delay: -1.0,
                    flee_time: 0.0,
                    flee_duration: FLEE_DURATION.0,
                });
            }
            self.ranges.push(SchoolRange {
                start,
                count: setting.count,
                phase: self.random.range(0.0, TAU),
                setting: school_index,
            });
        }
    }

    pub fn flee_from(&mut self, point: Vec2) {
        self.call_point = point;
        for fish in &mut self.fish {
            if fish.school_index >= VISIBLE_SCHOOL_COUNT {
                continue;
            }
            let distance = fish.position.sub(point).length();
            if distance > FLEE_REACTION_RADIUS {
                continue;
            }
            fish.flee_delay = distance / FLEE_PROPAGATION_SPEED
                + self.random.range(0.0, FLEE_RANDOM_DELAY);
            fish.flee_time = 0.0;
            fish.flee_duration = self.random.range(FLEE_DURATION.0, FLEE_DURATION.1);
        }
    }

    pub fn update(&mut self, dt: f32, time: f32) {
        for school_index in 0..VISIBLE_SCHOOL_COUNT.min(self.ranges.len()) {
            self.update_school(school_index, dt, time);
        }
    }

    fn update_school(&mut self, school_index: usize, dt: f32, time: f32) {
        let (start, count, range_phase, setting_index) = {
            let range = &self.ranges[school_index];
            (range.start, range.count, range.phase, range.setting)
        };
        let setting = &SCHOOLS[setting_index];

        let mut center = Vec2::default();
        let mut average_velocity = Vec2::default();
        for index in start..start + count {
            center = center.add(self.fish[index].position);
            average_velocity = average_velocity.add(self.fish[index].velocity);
        }
        let inverse = 1.0 / count as f32;
        center = center.mul(inverse);
        average_velocity = average_velocity.mul(inverse);
        let school_forward = average_velocity.normalize_or(from_angle(setting.heading));

        for index in start..start + count {
            let mut fish = self.fish[index];
            if fish.flee_delay >= 0.0 {
                fish.flee_delay -= dt;
                if fish.flee_delay <= 0.0 {
                    fish.flee_delay = -1.0;
                    fish.flee_time = fish.flee_duration;
                    let away = fish.position.sub(self.call_point).normalize_or(school_forward);
                    fish.velocity = fish.velocity.add(away.mul(FLEE_INITIAL_IMPULSE));
                }
            }

            let mut separation = Vec2::default();
            let mut alignment = Vec2::default();
            let mut cohesion = Vec2::default();
            let mut neighbours = 0;
            for other in start..start + count {
                if other == index {
                    continue;
                }
                let offset = fish.position.sub(self.fish[other].position);
                let distance = offset.length();
                if distance <= 0.001 || distance >= NEIGHBOUR_RADIUS {
                    continue;
                }
                neighbours += 1;
                cohesion = cohesion.add(self.fish[other].position);
                alignment = alignment.add(self.fish[other].velocity.normalize_or(school_forward));
                if distance < SEPARATION_RADIUS {
                    separation = separation.add(
                        offset
                            .normalize_or(school_forward)
                            .mul((SEPARATION_RADIUS - distance) / SEPARATION_RADIUS),
                    );
                }
            }

            let forward = fish.velocity.normalize_or(school_forward);
            let (cohesion, alignment) = if neighbours > 0 {
                (
                    cohesion.mul(1.0 / neighbours as f32).sub(fish.position).normalize_or(school_forward),
                    alignment.normalize_or(school_forward),
                )
            } else {
                (center.sub(fish.position).normalize_or(school_forward), school_forward)
            };

            let from_center = fish.position.sub(center).normalize_or(forward);
            let swirl = from_center.perpendicular().mul(setting.swirl_direction);
            let wander_angle = forward.y.atan2(forward.x)
                + (time * 0.62 + fish.phase + range_phase).sin() * 0.58
                + (time * 0.19 + fish.phase * 1.7).sin() * 0.31;
            let mut steering = forward
                .mul(0.82)
                .add(from_angle(wander_angle).mul(WANDER_STRENGTH));
            steering = steering.add(cohesion.mul(COHESION_STRENGTH));
            steering = steering.add(alignment.mul(ALIGNMENT_STRENGTH));
            steering = steering.add(separation.mul(SEPARATION_STRENGTH));
            steering = steering.add(swirl.mul(SWIRL_STRENGTH));

            let mut edge_force = Vec2::default();
            if fish.position.x < EDGE_MARGIN {
                edge_force.x += (EDGE_MARGIN - fish.position.x) / EDGE_MARGIN;
            }
            if fish.position.x > POND_W - EDGE_MARGIN {
                edge_force.x -= (fish.position.x - (POND_W - EDGE_MARGIN)) / EDGE_MARGIN;
            }
            if fish.position.y < EDGE_MARGIN {
                edge_force.y += (EDGE_MARGIN - fish.position.y) / EDGE_MARGIN;
            }
            if fish.position.y > POND_H - EDGE_MARGIN {
                edge_force.y -= (fish.position.y - (POND_H - EDGE_MARGIN)) / EDGE_MARGIN;
            }

            let mut target_speed =
                fish.cruise_speed * (1.0 + (time * 0.83 + fish.phase).sin() * SPEED_VARIATION);
            if fish.flee_time > 0.0 {
                fish.flee_time = (fish.flee_time - dt).max(0.0);
                let away = fish.position.sub(self.call_point).normalize_or(forward);
                let school_away = center.sub(self.call_point).normalize_or(away);
                steering = away.mul(FLEE_DIRECTION_STRENGTH).add(school_away.mul(FLEE_SCHOOLING_STRENGTH));
                steering = steering.add(alignment.mul(FLEE_SCHOOLING_STRENGTH));
                steering = steering.add(separation.mul(SEPARATION_STRENGTH));
                target_speed = self.random.range(FLEE_SPEED.0, FLEE_SPEED.1);
            }
            steering = steering.add(edge_force.mul(EDGE_STRENGTH));

            let desired = steering.normalize_or(forward);
            let current_heading = fish.velocity.y.atan2(fish.velocity.x);
            let desired_heading = desired.y.atan2(desired.x);
            let heading_step = clamp(
                wrap_angle(desired_heading - current_heading),
                -MAXIMUM_TURN_RATE * dt,
                MAXIMUM_TURN_RATE * dt,
            );
            let limited_direction = from_angle(current_heading + heading_step);
            let response = 1.0 - (-STEERING_RESPONSE * dt).exp();
            let current_speed = fish.velocity.length();
            let next_speed = current_speed + (target_speed - current_speed) * response;
            fish.velocity = limited_direction.mul(next_speed);
            fish.position = fish.position.add(fish.velocity.mul(dt));
            fish.tail_phase += (4.4 + next_speed * 0.16) * dt;

            self.fish[index] = fish;
        }
    }
}

pub struct TinyFrame {
    pub shadow: Vec<Vertex>,
    pub shapes: Vec<Vertex>,
    pub details: Vec<Vertex>,
}

impl TinyFrame {
    pub fn new() -> Self {
        Self {
            shadow: Vec::new(),
            shapes: Vec::new(),
            details: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.shadow.clear();
        self.shapes.clear();
        self.details.clear();
    }
}

pub fn draw(schools: &TinySchools, frame: &mut TinyFrame) {
    frame.clear();
    for fish in &schools.fish {
        if fish.school_index >= VISIBLE_SCHOOL_COUNT {
            continue;
        }
        let palette = &PALETTES[fish.palette % PALETTES.len()];
        let forward = fish.velocity.normalize_or(vec2(1.0, 0.0));
        let side = forward.perpendicular();
        draw_body(
            &mut frame.shadow,
            fish.position.add(SHADOW_OFFSET),
            forward,
            side,
            fish,
            None,
        );
        draw_body(
            &mut frame.shapes,
            fish.position,
            forward,
            side,
            fish,
            Some(palette),
        );

        let fin_root_forward = fish.position.add(forward.mul(fish.body_length * 0.02));
        let fin_back = fish.position.add(forward.mul(-fish.body_length * 0.18));
        for direction in [-1.0f32, 1.0] {
            let fin_tip = fin_root_forward
                .add(side.mul(fish.body_width * FIN_REACH_SCALE * direction));
            push_triangle(&mut frame.shapes, fin_root_forward, fin_tip, fin_back, palette.fin);
        }

        let stripe_front = fish.position.add(forward.mul(fish.body_length * 0.08));
        let stripe_back = fish.position.add(forward.mul(-fish.body_length * 0.09));
        let stripe_width = fish.body_width * 0.78;
        push_triangle(
            &mut frame.details,
            stripe_front.add(side.mul(stripe_width)),
            stripe_front.add(side.mul(-stripe_width)),
            stripe_back.add(side.mul(-stripe_width)),
            palette.accent,
        );
        push_triangle(
            &mut frame.details,
            stripe_front.add(side.mul(stripe_width)),
            stripe_back.add(side.mul(-stripe_width)),
            stripe_back.add(side.mul(stripe_width)),
            palette.accent,
        );

        let eye_forward = fish.position.add(forward.mul(fish.body_length * 0.31));
        for direction in [-1.0f32, 1.0] {
            push_circle(
                &mut frame.details,
                eye_forward.add(side.mul(fish.body_width * 0.58 * direction)),
                EYE_RADIUS,
                palette.eye,
                5,
            );
        }
    }
}

fn push_triangle(
    out: &mut Vec<Vertex>,
    a: Vec2,
    b: Vec2,
    c: Vec2,
    color: [f32; 3],
) {
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

fn draw_body(
    out: &mut Vec<Vertex>,
    center: Vec2,
    forward: Vec2,
    side: Vec2,
    fish: &TinyFish,
    palette: Option<&Palette>,
) {
    let nose = center.add(forward.mul(fish.body_length * 0.5));
    let front_left = center
        .add(forward.mul(fish.body_length * 0.16))
        .add(side.mul(fish.body_width));
    let front_right = center
        .add(forward.mul(fish.body_length * 0.16))
        .add(side.mul(-fish.body_width));
    let back_left = center
        .add(forward.mul(-fish.body_length * 0.34))
        .add(side.mul(fish.body_width * 0.58));
    let back_right = center
        .add(forward.mul(-fish.body_length * 0.34))
        .add(side.mul(-fish.body_width * 0.58));
    let tail_root = center.add(forward.mul(-fish.body_length * 0.44));

    let body_color = palette.map(|p| p.body).unwrap_or([1.0, 1.0, 1.0]);
    let front_color = palette.map(|p| p.light).unwrap_or([1.0, 1.0, 1.0]);
    let fin_color = palette.map(|p| p.fin).unwrap_or([1.0, 1.0, 1.0]);

    push_triangle(out, nose, front_left, center, front_color);
    push_triangle(out, nose, center, front_right, front_color);
    push_triangle(out, front_left, back_left, center, body_color);
    push_triangle(out, center, back_left, back_right, body_color);
    push_triangle(out, center, back_right, front_right, body_color);
    push_triangle(out, back_left, tail_root, back_right, body_color);

    let tail_length = fish.body_length * TAIL_LENGTH_SCALE;
    let tail_swing = fish.tail_phase.sin() * fish.body_width * 0.44;
    let tail_center = tail_root
        .add(forward.mul(-tail_length))
        .add(side.mul(tail_swing));
    let tail_half_width = fish.body_width * TAIL_WIDTH_SCALE;
    let upper = tail_center.add(side.mul(tail_half_width));
    let lower = tail_center.add(side.mul(-tail_half_width));
    let notch = tail_root
        .add(forward.mul(-tail_length * 0.62))
        .add(side.mul(tail_swing * 0.44));
    push_triangle(out, tail_root, upper, notch, fin_color);
    push_triangle(out, tail_root, notch, lower, fin_color);
}

pub const TINY_SHADOW_COLOR: [f32; 3] = SHADOW_COLOR;
