//! Everything the renderer needs that does not depend on the window system.

use std::time::{Duration, Instant};

use crate::butterflies;
use crate::duckweed;
use crate::fish::{self, KoiDraw};
use crate::gpu;
use crate::lotus;
use crate::sim::{FishConfig, School, FIXED_STEP};
use crate::tiny;
use crate::weather::Weather;

pub const FRAME_INTERVAL: Duration = Duration::from_millis(33);
pub const SIMULATED: f32 = FIXED_STEP;

pub struct Scene {
    pub school: School,
    pub draw: KoiDraw,
    pub lotus: lotus::Lotus,
    pub lotus_frame: lotus::LotusFrame,
    pub duckweed: duckweed::Duckweed,
    pub duckweed_frame: duckweed::DuckweedFrame,
    pub tiny_frame: tiny::TinyFrame,
    pub butterflies: butterflies::Butterflies,
    pub butterfly_frame: butterflies::ButterflyFrame,
    pub weather: Weather,
    pub config: FishConfig,
    accumulator: f32,
    sim_time: f32,
    last_sim: Instant,
}

impl Scene {
    pub fn new(config: FishConfig) -> Self {
        Self {
            school: School::new(config),
            draw: KoiDraw::new(),
            lotus: lotus::Lotus::new(),
            lotus_frame: lotus::LotusFrame::new(),
            duckweed: duckweed::Duckweed::new(),
            duckweed_frame: duckweed::DuckweedFrame::new(),
            tiny_frame: tiny::TinyFrame::new(),
            butterflies: butterflies::Butterflies::new(),
            butterfly_frame: butterflies::ButterflyFrame::new(),
            weather: Weather::new(),
            config,
            accumulator: 0.0,
            sim_time: 0.0,
            last_sim: Instant::now(),
        }
    }

    /// Advance the fixed-step simulation up to `now`.
    pub fn advance(&mut self, now: Instant) {
        let delta = (now - self.last_sim).as_secs_f32().min(0.1);
        self.last_sim = now;
        self.weather.update(delta);
        self.school
            .ripples
            .set_rain_intensity(self.weather.current().rain.clamp(0.0, 1.0));
        self.accumulator += delta;
        while self.accumulator >= FIXED_STEP {
            self.school.update(FIXED_STEP, self.sim_time);
            self.butterflies.update(FIXED_STEP, self.sim_time);
            self.sim_time += FIXED_STEP;
            self.accumulator -= FIXED_STEP;
        }
    }

    pub fn build_frame(&mut self, time: f32) {
        self.rebuild_fish();
        self.lotus
            .update(time, &mut self.lotus_frame);
        self.duckweed.update(time, &mut self.duckweed_frame);
        tiny::draw(&self.school.tiny, &mut self.tiny_frame);
        self.butterflies.draw(time, &mut self.butterfly_frame);
    }

    fn rebuild_fish(&mut self) {
        self.draw.clear();
        for index in 0..self.school.count {
            let mut fish = self.school.fish[index];
            let depth = fish::visual_depth(&self.config, fish.depth);
            let appearance =
                fish::depth_appearance(&fish::appearance_for(index), &self.config, depth);
            fish::build_render_spine(&mut fish);
            fish::draw_koi(&self.config, &fish, &appearance, &mut self.draw);
        }
    }

    pub fn ripple_uniforms(&self) -> ([[f32; 4]; 64], u32) {
        let mut ripple_data = [[0.0f32; 4]; 64];
        let mut ripple_count = 0;
        for ripple in &self.school.ripples.instances {
            if !ripple.alive || ripple.age < 0.0 || ripple_count >= 64 {
                continue;
            }
            ripple_data[ripple_count] = [
                ripple.center.x,
                ripple.center.y,
                ripple.age,
                ripple.kind.order_index() as f32 * 2.0 + ripple.sim_strength(),
            ];
            ripple_count += 1;
        }
        (ripple_data, ripple_count as u32)
    }

    pub fn frames(&self) -> (gpu::FishFrame<'_>, gpu::SurfaceFrame<'_>) {
        (
            gpu::FishFrame {
                outer: &self.draw.outer.vertices,
                body: &self.draw.body.vertices,
                shadow: &self.draw.shadow.vertices,
                disturbance: &self.draw.disturbance,
            },
            gpu::SurfaceFrame {
                shadow: &self.lotus_frame.shadow,
                leaves: &self.lotus_frame.leaves,
                veins: &self.lotus_frame.veins,
                flowers: &self.lotus_frame.flowers,
                duckweed_shadow: &self.duckweed_frame.shadow,
                duckweed_leaves: &self.duckweed_frame.leaves,
                duckweed_highlights: &self.duckweed_frame.highlights,
                tiny_shadow: &self.tiny_frame.shadow,
                tiny_shapes: &self.tiny_frame.shapes,
                tiny_details: &self.tiny_frame.details,
                butterfly_shadow: &self.butterfly_frame.shadow,
                butterflies: &self.butterfly_frame.shapes,
            },
        )
    }
}
