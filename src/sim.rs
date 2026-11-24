//! Port of the nagomi koi simulation (koi.ts, school.ts, ripple-system.ts).

pub const POND_W: f32 = 480.0;
pub const POND_H: f32 = 270.0;
pub const MAX_FISH: usize = 48;
pub const SPINE_NODES: usize = 14;
pub const MAX_RIPPLES: usize = 64;
pub const FIXED_STEP: f32 = 1.0 / 60.0;
pub const TAU: f32 = std::f32::consts::TAU;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

pub const fn vec2(x: f32, y: f32) -> Vec2 {
    Vec2 { x, y }
}

impl Vec2 {
    pub fn add(self, other: Vec2) -> Vec2 {
        vec2(self.x + other.x, self.y + other.y)
    }
    pub fn sub(self, other: Vec2) -> Vec2 {
        vec2(self.x - other.x, self.y - other.y)
    }
    pub fn mul(self, scalar: f32) -> Vec2 {
        vec2(self.x * scalar, self.y * scalar)
    }
    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }
    pub fn normalize_or(self, fallback: Vec2) -> Vec2 {
        let magnitude = self.length();
        if magnitude > 0.0001 {
            self.mul(1.0 / magnitude)
        } else {
            fallback
        }
    }
    pub fn perpendicular(self) -> Vec2 {
        vec2(-self.y, self.x)
    }
    pub fn lerp(self, other: Vec2, amount: f32) -> Vec2 {
        self.add(other.sub(self).mul(amount))
    }
}

pub fn from_angle(angle: f32) -> Vec2 {
    vec2(angle.cos(), angle.sin())
}

pub fn clamp(value: f32, low: f32, high: f32) -> f32 {
    value.max(low).min(high)
}

pub fn wrap_angle(angle: f32) -> f32 {
    angle.sin().atan2(angle.cos())
}

#[derive(Clone, Copy)]
pub struct XorShift32 {
    pub state: u32,
}

impl XorShift32 {
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }
    pub fn next(&mut self) -> u32 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        self.state = value;
        self.state
    }
    pub fn unit(&mut self) -> f32 {
        (self.next() & 0x00ff_ffff) as f32 / 0x0100_0000 as f32
    }
    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }
}

// ---- defaults from src/settings/definition.ts ---------------------------------

#[derive(Clone, Copy)]
pub struct FishConfig {
    pub initial_count: usize,
    pub regular_length: (f32, f32),
    pub tiny_every: usize,
    pub tiny_length: (f32, f32),
    pub regular_width_ratio: (f32, f32),
    pub tiny_width_ratio: (f32, f32),
    pub shadow_surface_opacity: f32,
    pub shadow_deep_opacity: f32,
    pub shadow_offset: Vec2,
    pub shadow_depth_offset: Vec2,
    pub depth_initial_range: (f32, f32),
    pub depth_shallow_range: (f32, f32),
    pub depth_deep_range: (f32, f32),
    pub depth_surface_duration: (f32, f32),
    pub depth_deep_duration: (f32, f32),
    pub depth_change_probability: f32,
    pub depth_transition: (f32, f32),
    pub call_rise_depth: f32,
    pub call_rise_seconds: f32,
    pub depth_visual_start: f32,
    pub depth_visual_end: f32,
    pub deep_brightness: f32,
    pub deep_saturation: f32,
    pub deep_water_tint: [f32; 3],
    pub distortion_strength: f32,
    pub distortion_length_scale: f32,
    pub distortion_width_scale: f32,
    pub distortion_wave_frequency: f32,
    pub distortion_wave_speed: f32,
    pub feeding_interval: (f32, f32),
    pub feeding_retry: (f32, f32),
    pub feeding_eligible_depth: f32,
    pub feeding_eligible_speed_fraction: f32,
    pub feeding_mouth_forward_offset: f32,
    pub feeding_animation_seconds: f32,
    pub call_minimum_delay: f32,
    pub call_distance_at_maximum_delay: f32,
    pub call_maximum_distance_delay: f32,
    pub call_distance_exponent: f32,
    pub call_random_jitter: f32,
    pub call_temperament_delay: f32,
    pub call_target_lifetime: f32,
    pub call_chase_boost_seconds: f32,
    pub call_chase_speed_multiplier: f32,
    pub call_initial_extra_speed_multiplier: f32,
}

impl Default for FishConfig {
    fn default() -> Self {
        Self {
            initial_count: 14,
            regular_length: (27.0, 40.0),
            tiny_every: 2,
            tiny_length: (16.0, 22.0),
            regular_width_ratio: (0.17, 0.2),
            tiny_width_ratio: (0.18, 0.22),
            shadow_surface_opacity: 0.38,
            shadow_deep_opacity: 0.56,
            shadow_offset: vec2(4.4, 10.4),
            shadow_depth_offset: vec2(-3.0, -7.0),
            depth_initial_range: (0.05, 0.18),
            depth_shallow_range: (0.04, 0.22),
            depth_deep_range: (0.45, 0.75),
            depth_surface_duration: (8.0, 22.0),
            depth_deep_duration: (4.0, 10.0),
            depth_change_probability: 0.72,
            depth_transition: (2.0, 5.0),
            call_rise_depth: 0.035,
            call_rise_seconds: 2.4,
            depth_visual_start: 0.1,
            depth_visual_end: 0.72,
            deep_brightness: 0.59,
            deep_saturation: 0.76,
            deep_water_tint: [0.66, 0.84, 0.8],
            distortion_strength: 2.3,
            distortion_length_scale: 0.72,
            distortion_width_scale: 1.85,
            distortion_wave_frequency: 8.5,
            distortion_wave_speed: 2.4,
            feeding_interval: (1.0, 4.0),
            feeding_retry: (0.55, 1.35),
            feeding_eligible_depth: 0.23,
            feeding_eligible_speed_fraction: 0.62,
            feeding_mouth_forward_offset: 0.66,
            feeding_animation_seconds: 0.24,
            call_minimum_delay: 0.04,
            call_distance_at_maximum_delay: 360.0,
            call_maximum_distance_delay: 1.05,
            call_distance_exponent: 1.5,
            call_random_jitter: 0.18,
            call_temperament_delay: 0.22,
            call_target_lifetime: 4.4,
            call_chase_boost_seconds: 2.6,
            call_chase_speed_multiplier: 2.3,
            call_initial_extra_speed_multiplier: 0.34,
        }
    }
}

#[derive(Clone, Copy)]
pub struct RippleProfile {
    pub maximum_active: usize,
    pub ripples_per_event: usize,
    pub interval_seconds: f32,
    pub initial_strength: f32,
    pub strength_falloff: f32,
    pub lifetime: f32,
    pub start_radius: f32,
    pub expansion_speed: f32,
    pub distortion: f32,
    pub band_sharpness: f32,
    pub fade_start: f32,
    pub strength_decay: f32,
}

pub const TOUCH_RIPPLE: RippleProfile = RippleProfile {
    maximum_active: 16,
    ripples_per_event: 5,
    interval_seconds: 0.023,
    initial_strength: 1.0,
    strength_falloff: 0.92,
    lifetime: 2.2,
    start_radius: 4.0,
    expansion_speed: 62.0,
    distortion: 5.55,
    band_sharpness: 0.3,
    fade_start: 0.58,
    strength_decay: 0.18,
};

pub const RAIN_RIPPLE: RippleProfile = RippleProfile {
    maximum_active: 48,
    ripples_per_event: 3,
    interval_seconds: 0.103,
    initial_strength: 1.55,
    strength_falloff: 1.0,
    lifetime: 1.25,
    start_radius: 0.8,
    expansion_speed: 30.0,
    distortion: 4.8,
    band_sharpness: 0.62,
    fade_start: 0.34,
    strength_decay: 0.18,
};

pub const MOUTH_RIPPLE: RippleProfile = RippleProfile {
    maximum_active: 10,
    ripples_per_event: 1,
    interval_seconds: 0.08,
    initial_strength: 0.5,
    strength_falloff: 0.65,
    lifetime: 0.9,
    start_radius: 1.2,
    expansion_speed: 17.0,
    distortion: 10.4,
    band_sharpness: 0.7,
    fade_start: 0.25,
    strength_decay: 0.45,
};

pub const RAIN_DROPS_PER_SECOND: f32 = 12.0;
pub const RAIN_FREQUENCY_VARIATION: f32 = 0.35;
pub const RAIN_MAXIMUM_DROPS_PER_FRAME: usize = 40;
pub const RAIN_EDGE_MARGIN: f32 = 5.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RippleType {
    Rain,
    Mouth,
    Touch,
}

impl RippleType {
    pub fn order_index(self) -> usize {
        match self {
            RippleType::Touch => 0,
            RippleType::Rain => 1,
            RippleType::Mouth => 2,
        }
    }
    pub fn priority(self) -> u8 {
        match self {
            RippleType::Rain => 0,
            RippleType::Mouth => 1,
            RippleType::Touch => 2,
        }
    }
    pub fn profile(self) -> &'static RippleProfile {
        match self {
            RippleType::Touch => &TOUCH_RIPPLE,
            RippleType::Rain => &RAIN_RIPPLE,
            RippleType::Mouth => &MOUTH_RIPPLE,
        }
    }
}

impl RippleInstance {
    pub fn sim_strength(&self) -> f32 {
        self.strength.max(0.0).min(1.999)
    }
}

#[derive(Clone, Copy)]
pub struct RippleInstance {
    pub center: Vec2,
    pub age: f32,
    pub strength: f32,
    pub kind: RippleType,
    pub alive: bool,
}

pub struct RippleSystem {
    pub instances: Vec<RippleInstance>,
    random: XorShift32,
    rain_intensity: f32,
    rain_countdown: f32,
}

impl RippleSystem {
    pub fn new() -> Self {
        Self {
            instances: vec![
                RippleInstance {
                    center: vec2(0.0, 0.0),
                    age: 0.0,
                    strength: 0.0,
                    kind: RippleType::Touch,
                    alive: false,
                };
                MAX_RIPPLES
            ],
            random: XorShift32::new(0x7a11_fa11),
            rain_intensity: 0.0,
            rain_countdown: 0.0,
        }
    }

    pub fn trigger(&mut self, kind: RippleType, point: Vec2) {
        let profile = kind.profile();
        let ripple_count = profile.ripples_per_event.min(profile.maximum_active);
        for index in 0..ripple_count {
            let Some(slot) = self.allocate(kind) else { break };
            let strength =
                profile.initial_strength * profile.strength_falloff.powi(index as i32);
            let ripple = &mut self.instances[slot];
            ripple.center = point;
            ripple.age = -(index as f32) * profile.interval_seconds;
            ripple.strength = strength;
            ripple.kind = kind;
            ripple.alive = true;
        }
    }

    pub fn set_rain_intensity(&mut self, intensity: f32) {
        self.rain_intensity = clamp(intensity, 0.0, 1.0);
        if self.rain_intensity <= 0.0 {
            self.rain_countdown = 0.0;
        }
    }

    pub fn update(&mut self, delta: f32, width: f32, height: f32) {
        for ripple in &mut self.instances {
            if !ripple.alive {
                continue;
            }
            ripple.age += delta;
            if ripple.age > ripple.kind.profile().lifetime {
                ripple.alive = false;
            }
        }

        if self.rain_intensity <= 0.0 {
            return;
        }
        self.rain_countdown -= delta;
        let mut emitted = 0;
        while self.rain_countdown <= 0.0 && emitted < RAIN_MAXIMUM_DROPS_PER_FRAME {
            let x = self
                .random
                .range(RAIN_EDGE_MARGIN, width - RAIN_EDGE_MARGIN);
            let y = self
                .random
                .range(RAIN_EDGE_MARGIN, height - RAIN_EDGE_MARGIN);
            self.trigger(RippleType::Rain, vec2(x, y));
            let average_interval = 1.0 / RAIN_DROPS_PER_SECOND.max(0.1);
            let variation = RAIN_FREQUENCY_VARIATION.max(0.0);
            let interval_multiplier = self.random.range(
                (1.0 - variation).max(0.05),
                1.0 + variation,
            );
            self.rain_countdown +=
                average_interval * interval_multiplier / self.rain_intensity.max(0.12);
            emitted += 1;
        }
    }

    fn allocate(&mut self, kind: RippleType) -> Option<usize> {
        let maximum_active = kind.profile().maximum_active;
        let mut active_of_type = 0;
        let mut oldest_of_type: Option<usize> = None;
        let mut available: Option<usize> = None;

        for (index, ripple) in self.instances.iter().enumerate() {
            if !ripple.alive {
                available = available.or(Some(index));
                continue;
            }
            if ripple.kind != kind {
                continue;
            }
            active_of_type += 1;
            if oldest_of_type
                .map(|oldest| ripple.age > self.instances[oldest].age)
                .unwrap_or(true)
            {
                oldest_of_type = Some(index);
            }
        }

        if active_of_type >= maximum_active {
            return oldest_of_type;
        }
        if available.is_some() {
            return available;
        }

        let mut replacement: Option<usize> = None;
        let mut replacement_priority = u8::MAX;
        for (index, ripple) in self.instances.iter().enumerate() {
            let priority = ripple.kind.priority();
            if priority > kind.priority() {
                continue;
            }
            if priority < replacement_priority
                || (priority == replacement_priority
                    && replacement
                        .map(|current| ripple.age > self.instances[current].age)
                        .unwrap_or(true))
            {
                replacement = Some(index);
                replacement_priority = priority;
            }
        }
        replacement
    }
}

// ---- koi ----------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SwimState {
    Glide,
    Coast,
    Hover,
    Burst,
    Pivot,
}

#[derive(Clone, Copy)]
pub struct Koi {
    pub position: Vec2,
    pub velocity: Vec2,
    pub spine: [Vec2; SPINE_NODES],
    pub render_spine: [Vec2; SPINE_NODES],
    pub heading: f32,
    pub angular_velocity: f32,
    pub speed: f32,
    pub cruise_speed: f32,
    pub maximum_speed: f32,
    pub turn_strength: f32,
    pub body_length: f32,
    pub body_width: f32,
    pub swim_phase: f32,
    pub phase_offset: f32,
    pub wander_seed: f32,
    pub state_age: f32,
    pub state_duration: f32,
    pub pivot_heading: f32,
    pub reactivity: f32,
    pub call_delay: f32,
    pub responded_to_call: bool,
    pub call_response_age: f32,
    pub tail_effort: f32,
    pub depth: f32,
    pub target_depth: f32,
    pub depth_transition_rate: f32,
    pub depth_state_age: f32,
    pub depth_state_duration: f32,
    pub in_deep_period: bool,
    pub gulp_countdown: f32,
    pub gulp_animation: f32,
    pub behavior_rng: u32,
    pub state: SwimState,
}

impl Default for Koi {
    fn default() -> Self {
        Self {
            position: vec2(0.0, 0.0),
            velocity: vec2(0.0, 0.0),
            spine: [Vec2::default(); SPINE_NODES],
            render_spine: [Vec2::default(); SPINE_NODES],
            heading: 0.0,
            angular_velocity: 0.0,
            speed: 0.0,
            cruise_speed: 20.0,
            maximum_speed: 34.0,
            turn_strength: 5.0,
            body_length: 24.0,
            body_width: 4.0,
            swim_phase: 0.0,
            phase_offset: 0.0,
            wander_seed: 0.0,
            state_age: 0.0,
            state_duration: 2.0,
            pivot_heading: 0.0,
            reactivity: 0.7,
            call_delay: 0.0,
            responded_to_call: false,
            call_response_age: 0.0,
            tail_effort: 0.6,
            depth: 0.08,
            target_depth: 0.08,
            depth_transition_rate: 1.0,
            depth_state_age: 0.0,
            depth_state_duration: 10.0,
            in_deep_period: false,
            gulp_countdown: 8.0,
            gulp_animation: 0.0,
            behavior_rng: 1,
            state: SwimState::Glide,
        }
    }
}

impl Koi {
    pub fn reset(&mut self, index: usize, config: &FishConfig, random: &mut XorShift32) {
        self.position = vec2(
            random.range(45.0, POND_W - 45.0),
            random.range(32.0, POND_H - 32.0),
        );
        self.heading = random.range(-std::f32::consts::PI, std::f32::consts::PI);
        self.cruise_speed = random.range(13.0, 21.0);
        self.maximum_speed = self.cruise_speed * random.range(1.55, 1.9);
        self.speed = self.cruise_speed * random.range(0.72, 1.05);
        self.turn_strength = random.range(4.4, 6.8);
        let tiny = index % config.tiny_every == config.tiny_every - 1;
        let length_range = if tiny {
            config.tiny_length
        } else {
            config.regular_length
        };
        let width_range = if tiny {
            config.tiny_width_ratio
        } else {
            config.regular_width_ratio
        };
        self.body_length = random.range(length_range.0, length_range.1);
        self.body_width = self.body_length * random.range(width_range.0, width_range.1);
        self.phase_offset = random.range(0.0, TAU);
        self.swim_phase = self.phase_offset;
        self.wander_seed = random.range(0.0, 100.0);
        self.reactivity = random.range(0.35, 1.0);
        self.call_delay = 0.0;
        self.responded_to_call = false;
        self.call_response_age = 0.0;
        self.behavior_rng = 0x9e37_79b9 ^ (index as u32 + 1).wrapping_mul(0x85eb_ca6b);
        self.depth = random.range(
            config.depth_initial_range.0,
            config.depth_initial_range.1,
        );
        self.target_depth = self.depth;
        self.depth_transition_rate = 3.0
            / random.range(config.depth_transition.0, config.depth_transition.1);
        self.depth_state_duration = random.range(
            config.depth_surface_duration.0,
            config.depth_surface_duration.1,
        );
        self.depth_state_age = random.range(0.0, self.depth_state_duration * 0.7);
        self.in_deep_period = false;
        self.gulp_countdown = random.range(
            config.feeding_interval.0,
            config.feeding_interval.1,
        );
        self.gulp_animation = 0.0;
        self.state = match index % 5 {
            0 => SwimState::Glide,
            1 => SwimState::Coast,
            2 => SwimState::Hover,
            3 => SwimState::Burst,
            _ => SwimState::Pivot,
        };

        let durations = [
            (1.7, 4.8),
            (0.8, 2.0),
            (0.7, 2.9),
            (0.35, 0.9),
            (0.35, 0.8),
        ][state_index(self.state)];
        self.state_duration = random.range(durations.0, durations.1);
        self.state_age = random.range(0.0, self.state_duration * 0.8);
        self.pivot_heading = self.heading;
        self.tail_effort = 0.6;
        self.angular_velocity = 0.0;
        self.velocity = from_angle(self.heading).mul(self.speed);

        let backward = from_angle(self.heading).mul(-self.body_length / (SPINE_NODES - 1) as f32);
        for node in 0..SPINE_NODES {
            self.spine[node] = self.position.add(backward.mul(node as f32));
            self.render_spine[node] = self.spine[node];
        }
    }
}

fn state_index(state: SwimState) -> usize {
    match state {
        SwimState::Glide => 0,
        SwimState::Coast => 1,
        SwimState::Hover => 2,
        SwimState::Burst => 3,
        SwimState::Pivot => 4,
    }
}

pub struct School {
    pub fish: Vec<Koi>,
    pub ripples: RippleSystem,
    pub tiny: crate::tiny::TinySchools,
    pub count: usize,
    pub target_active: bool,
    config: FishConfig,
    random: XorShift32,
    target: Vec2,
    target_age: f32,
}

impl School {
    pub fn new(config: FishConfig) -> Self {
        let mut school = Self {
            fish: vec![Koi::default(); MAX_FISH],
            ripples: RippleSystem::new(),
            tiny: crate::tiny::TinySchools::new(),
            count: config.initial_count,
            target_active: false,
            config,
            random: XorShift32::new(0x00c0_ffee),
            target: vec2(POND_W * 0.5, POND_H * 0.5),
            target_age: 0.0,
        };
        for index in 0..MAX_FISH {
            let mut fish = school.fish[index];
            fish.reset(index, &school.config, &mut school.random);
            school.fish[index] = fish;
        }
        school
    }

    pub fn call_to(&mut self, point: Vec2) {
        self.target = point;
        self.target_active = true;
        self.target_age = 0.0;
        let response = &self.config;
        for index in 0..self.count {
            let fish = &mut self.fish[index];
            let distance_to_call = fish.position.sub(point).length();
            let distance_amount = clamp(
                distance_to_call / response.call_distance_at_maximum_delay,
                0.0,
                1.0,
            )
            .powf(response.call_distance_exponent);
            fish.call_delay = response.call_minimum_delay
                + distance_amount * response.call_maximum_distance_delay
                + self.random.range(0.0, response.call_random_jitter)
                + (1.0 - fish.reactivity) * response.call_temperament_delay;
            fish.responded_to_call = false;
            fish.call_response_age = 0.0;
        }
        self.tiny.flee_from(point);
        self.ripples.trigger(RippleType::Touch, point);
    }

    pub fn scatter(&mut self) {
        for index in 0..self.count {
            let mut fish = self.fish[index];
            fish.heading += self.random.range(-1.35, 1.35);
            fish.speed = fish.maximum_speed;
            fish.angular_velocity += self.random.range(-2.0, 2.0);
            self.enter_state(&mut fish, SwimState::Burst);
            self.fish[index] = fish;
        }
        self.target_active = false;
    }

    pub fn update(&mut self, delta: f32, time: f32) {
        self.target_age += delta;
        if self.target_active && self.target_age > self.config.call_target_lifetime {
            self.target_active = false;
        }

        let mut desired: Vec<Vec2> = Vec::with_capacity(self.count);
        let mut desired_speed: Vec<f32> = Vec::with_capacity(self.count);
        for index in 0..self.count {
            let mut fish = self.fish[index];
            fish.call_delay = (fish.call_delay - delta).max(0.0);
            if self.target_active && fish.responded_to_call {
                fish.call_response_age += delta;
            }
            if self.target_active && fish.call_delay <= 0.0 && !fish.responded_to_call {
                fish.responded_to_call = true;
                fish.call_response_age = 0.0;
                fish.target_depth = self.config.call_rise_depth;
                fish.depth_transition_rate =
                    3.0 / self.config.call_rise_seconds.max(0.1);
                self.enter_state(&mut fish, SwimState::Burst);
            }
            self.update_natural_state(&mut fish, delta);
            self.update_depth(&mut fish, delta);
            self.update_feeding(&mut fish, delta);
            desired.push(self.steering_for(index, time));
            desired_speed.push(self.desired_speed_for(index));
            self.fish[index] = fish;
        }
        for index in 0..self.count {
            let mut fish = self.fish[index];
            self.integrate(&mut fish, desired[index], desired_speed[index], delta);
            self.fish[index] = fish;
        }

        self.tiny.update(delta, time);
        self.ripples.update(delta, POND_W, POND_H);
    }

    fn update_depth(&mut self, fish: &mut Koi, delta: f32) {
        fish.depth_state_age += delta;
        let rising_for_call = self.target_active && fish.responded_to_call;
        if !rising_for_call && fish.depth_state_age >= fish.depth_state_duration {
            fish.depth_state_age = 0.0;
            if self.behavior_unit(fish) < self.config.depth_change_probability {
                fish.in_deep_period = !fish.in_deep_period;
            }
            let range = if fish.in_deep_period {
                self.config.depth_deep_range
            } else {
                self.config.depth_shallow_range
            };
            let durations = if fish.in_deep_period {
                self.config.depth_deep_duration
            } else {
                self.config.depth_surface_duration
            };
            fish.target_depth = self.behavior_range(fish, range.0, range.1);
            fish.depth_state_duration = self.behavior_range(fish, durations.0, durations.1);
            let transition_seconds = self.behavior_range(
                fish,
                self.config.depth_transition.0,
                self.config.depth_transition.1,
            );
            fish.depth_transition_rate = 3.0 / transition_seconds.max(0.1);
        }

        fish.depth +=
            (fish.target_depth - fish.depth) * (1.0 - (-fish.depth_transition_rate * delta).exp());
    }

    fn update_feeding(&mut self, fish: &mut Koi, delta: f32) {
        fish.gulp_animation = (fish.gulp_animation - delta).max(0.0);
        fish.gulp_countdown -= delta;
        if fish.gulp_countdown > 0.0 {
            return;
        }

        let calm_state = matches!(
            fish.state,
            SwimState::Hover | SwimState::Coast | SwimState::Glide
        );
        let chasing = self.target_active && fish.responded_to_call;
        let eligible = calm_state
            && !chasing
            && fish.depth <= self.config.feeding_eligible_depth
            && fish.speed <= fish.cruise_speed * self.config.feeding_eligible_speed_fraction;

        if !eligible {
            fish.gulp_countdown = self.behavior_range(
                fish,
                self.config.feeding_retry.0,
                self.config.feeding_retry.1,
            );
            return;
        }

        let forward = vec2(fish.heading.cos(), fish.heading.sin());
        let mouth_distance = fish.body_width * self.config.feeding_mouth_forward_offset;
        self.ripples.trigger(
            RippleType::Mouth,
            vec2(
                fish.position.x + forward.x * mouth_distance,
                fish.position.y + forward.y * mouth_distance,
            ),
        );
        fish.gulp_animation = self.config.feeding_animation_seconds;
        fish.gulp_countdown = self.behavior_range(
            fish,
            self.config.feeding_interval.0,
            self.config.feeding_interval.1,
        );
    }

    fn behavior_unit(&self, fish: &mut Koi) -> f32 {
        let mut value = fish.behavior_rng;
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        fish.behavior_rng = value;
        (fish.behavior_rng & 0x00ff_ffff) as f32 / 0x0100_0000 as f32
    }

    fn behavior_range(&self, fish: &mut Koi, low: f32, high: f32) -> f32 {
        low + (high - low) * self.behavior_unit(fish)
    }

    fn enter_state(&mut self, fish: &mut Koi, next: SwimState) {
        fish.state = next;
        fish.state_age = 0.0;
        match next {
            SwimState::Glide => fish.state_duration = self.behavior_range(fish, 1.7, 5.2),
            SwimState::Coast => fish.state_duration = self.behavior_range(fish, 0.7, 2.1),
            SwimState::Hover => fish.state_duration = self.behavior_range(fish, 0.65, 3.1),
            SwimState::Burst => fish.state_duration = self.behavior_range(fish, 0.32, 0.92),
            SwimState::Pivot => {
                fish.state_duration = self.behavior_range(fish, 0.3, 0.78);
                let direction = if self.behavior_unit(fish) < 0.5 { -1.0 } else { 1.0 };
                fish.pivot_heading = wrap_angle(
                    fish.heading + direction * self.behavior_range(fish, 0.85, 2.35),
                );
            }
        }
    }

    fn update_natural_state(&mut self, fish: &mut Koi, delta: f32) {
        fish.state_age += delta;
        if fish.state_age < fish.state_duration {
            return;
        }

        let roll = self.behavior_unit(fish);
        match fish.state {
            SwimState::Glide => {
                if roll < 0.25 {
                    self.enter_state(fish, SwimState::Coast);
                } else if roll < 0.43 {
                    self.enter_state(fish, SwimState::Hover);
                } else if roll < 0.61 {
                    self.enter_state(fish, SwimState::Pivot);
                } else if roll < 0.72 {
                    self.enter_state(fish, SwimState::Burst);
                } else {
                    self.enter_state(fish, SwimState::Glide);
                }
            }
            SwimState::Coast => {
                if roll < 0.38 {
                    self.enter_state(fish, SwimState::Hover);
                } else if roll < 0.72 {
                    self.enter_state(fish, SwimState::Glide);
                } else if roll < 0.9 {
                    self.enter_state(fish, SwimState::Pivot);
                } else {
                    self.enter_state(fish, SwimState::Burst);
                }
            }
            SwimState::Hover => {
                if roll < 0.34 {
                    self.enter_state(fish, SwimState::Pivot);
                } else if roll < 0.55 {
                    self.enter_state(fish, SwimState::Burst);
                } else {
                    self.enter_state(fish, SwimState::Glide);
                }
            }
            SwimState::Burst => self.enter_state(fish, SwimState::Coast),
            SwimState::Pivot => {
                let next = if roll < 0.38 {
                    SwimState::Burst
                } else {
                    SwimState::Glide
                };
                self.enter_state(fish, next);
            }
        }
    }

    fn steering_for(&self, index: usize, time: f32) -> Vec2 {
        let fish = &self.fish[index];
        let forward = from_angle(fish.heading);
        let mut steering = forward.mul(0.95);

        if fish.state == SwimState::Pivot {
            steering = from_angle(fish.pivot_heading).mul(4.7);
        } else if fish.state != SwimState::Hover {
            let wander = (time * 0.29 + fish.wander_seed).sin() * 0.7
                + (time * 0.113 + fish.wander_seed * 1.73).sin() * 0.45;
            steering = steering.add(from_angle(fish.heading + wander).mul(0.62));
        }

        let mut separation = vec2(0.0, 0.0);
        let mut alignment = vec2(0.0, 0.0);
        let mut cohesion = vec2(0.0, 0.0);
        let mut neighbours = 0;

        for other in 0..self.count {
            if other == index {
                continue;
            }
            let offset = fish.position.sub(self.fish[other].position);
            let distance = offset.length();
            if distance > 0.001 && distance < 37.0 {
                neighbours += 1;
                cohesion = cohesion.add(self.fish[other].position);
                alignment = alignment.add(self.fish[other].velocity.normalize_or(forward));
                if distance < 14.0 {
                    separation = separation.add(
                        offset.normalize_or(forward).mul((14.0 - distance) / 14.0),
                    );
                }
            }
        }

        if neighbours > 0 {
            let inverse = 1.0 / neighbours as f32;
            cohesion = cohesion.mul(inverse).sub(fish.position).normalize_or(forward);
            alignment = alignment.normalize_or(forward);
            steering = steering.add(cohesion.mul(0.25));
            steering = steering.add(alignment.mul(0.42));
            steering = steering.add(separation.mul(2.8));
        }

        let margin = 32.0;
        let mut edge_force = vec2(0.0, 0.0);
        if fish.position.x < margin {
            edge_force.x += (margin - fish.position.x) / margin;
        }
        if fish.position.x > POND_W - margin {
            edge_force.x -= (fish.position.x - (POND_W - margin)) / margin;
        }
        if fish.position.y < margin {
            edge_force.y += (margin - fish.position.y) / margin;
        }
        if fish.position.y > POND_H - margin {
            edge_force.y -= (fish.position.y - (POND_H - margin)) / margin;
        }
        steering = steering.add(edge_force.mul(4.8));

        if self.target_active && fish.call_delay <= 0.0 {
            let to_target = self.target.sub(fish.position);
            let distance = to_target.length();
            if distance > 13.0 {
                let chase_pull = if fish.call_response_age < self.config.call_chase_boost_seconds
                {
                    3.35
                } else {
                    2.45
                };
                steering = steering.add(to_target.normalize_or(forward).mul(chase_pull));
            } else {
                let target_direction = to_target.normalize_or(forward);
                steering = steering.add(target_direction.perpendicular().mul(2.2));
                steering = steering.add(target_direction.mul(-0.5));
            }
        }

        steering.normalize_or(forward)
    }

    fn desired_speed_for(&self, index: usize) -> f32 {
        let fish = &self.fish[index];
        let chase_duration = self.config.call_chase_boost_seconds;
        let chasing = self.target_active
            && fish.responded_to_call
            && fish.call_response_age < chase_duration;
        if chasing {
            let chase_fade = 1.0 - clamp(fish.call_response_age / chase_duration, 0.0, 1.0);
            return fish.maximum_speed
                * (self.config.call_chase_speed_multiplier
                    + chase_fade * self.config.call_initial_extra_speed_multiplier);
        }

        let mut intention = fish.cruise_speed;
        if self.target_active && fish.call_delay <= 0.0 {
            let distance = self.target.sub(fish.position).length();
            let urgency = clamp(distance / 105.0, 0.2, 1.0);
            intention = fish.cruise_speed + (fish.maximum_speed - fish.cruise_speed) * urgency;
        }

        match fish.state {
            SwimState::Glide => intention,
            SwimState::Coast => intention * 0.28,
            SwimState::Hover => 0.0,
            SwimState::Burst => fish.maximum_speed * 1.08,
            SwimState::Pivot => fish.cruise_speed * 0.16,
        }
    }

    fn integrate(&self, fish: &mut Koi, desired: Vec2, desired_speed: f32, delta: f32) {
        let desired_heading = desired.y.atan2(desired.x);
        let heading_error = wrap_angle(desired_heading - fish.heading);
        let pivoting = fish.state == SwimState::Pivot;
        let turn_multiplier = if pivoting { 2.65 } else { 1.0 };
        let angular_damping = if pivoting { 2.15 } else { 3.8 };
        let angular_acceleration = heading_error * fish.turn_strength * turn_multiplier
            - fish.angular_velocity * angular_damping;
        fish.angular_velocity += angular_acceleration * delta;
        let maximum_turn_rate = if pivoting { 4.35 } else { 2.25 };
        fish.angular_velocity =
            clamp(fish.angular_velocity, -maximum_turn_rate, maximum_turn_rate);
        fish.heading = wrap_angle(fish.heading + fish.angular_velocity * delta);

        let (speed_response, desired_tail_effort) = match fish.state {
            SwimState::Glide => (1.65, 0.62),
            SwimState::Coast => (1.05, 0.16),
            SwimState::Hover => (3.6, 0.05),
            SwimState::Burst => (6.4, 1.22),
            SwimState::Pivot => (4.2, 1.0),
        };

        fish.speed +=
            (desired_speed - fish.speed) * (1.0 - (-speed_response * delta).exp());
        fish.tail_effort +=
            (desired_tail_effort - fish.tail_effort) * (1.0 - (-4.5 * delta).exp());
        fish.velocity = from_angle(fish.heading).mul(fish.speed);
        fish.position = fish.position.add(fish.velocity.mul(delta));

        let beat_rate =
            0.45 + (fish.speed / fish.maximum_speed) * 4.6 + fish.tail_effort * 0.9;
        fish.swim_phase += beat_rate * delta;

        fish.spine[0] = fish.position;
        let spacing = fish.body_length / (SPINE_NODES - 1) as f32;
        for node in 1..SPINE_NODES {
            let fallback = from_angle(fish.heading).mul(-1.0);
            let direction = fish.spine[node]
                .sub(fish.spine[node - 1])
                .normalize_or(fallback);
            let constrained = fish.spine[node - 1].add(direction.mul(spacing));
            let tail_amount = node as f32 / (SPINE_NODES - 1) as f32;
            let stiffness = 0.94 - tail_amount * 0.17;
            fish.spine[node] = fish.spine[node].lerp(constrained, stiffness);
        }
    }
}
