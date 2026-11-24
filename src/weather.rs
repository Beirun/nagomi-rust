//! Weather presets ported from weather.ts, with the same 2.25/s blend.

#[derive(Clone, Copy)]
pub struct BedParams {
    pub deep: [f32; 3],
    pub shallow: [f32; 3],
    pub vertical_tone: f32,
    pub edge_darkening: f32,
}

#[derive(Clone, Copy)]
pub struct WaterParams {
    pub tint: [f32; 3],
    pub large: [f32; 3],
    pub large_core: [f32; 3],
    pub large_cell: f32,
    pub large_opacity: f32,
    pub large_speed: f32,
    pub secondary: [f32; 3],
    pub secondary_core: [f32; 3],
    pub secondary_cell: f32,
    pub secondary_opacity: f32,
    pub secondary_speed: f32,
    pub detail: [f32; 3],
    pub detail_core: [f32; 3],
    pub detail_cell: f32,
    pub detail_opacity: f32,
    pub detail_speed: f32,
}

#[derive(Clone, Copy)]
pub struct WeatherParams {
    pub tint: [f32; 3],
    pub brightness: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub vignette: f32,
    pub cloud_strength: f32,
    pub light_color: [f32; 3],
    pub light_strength: f32,
    pub light_direction: [f32; 2],
    pub bed: BedParams,
    pub water: WaterParams,
    pub shadow_color: [f32; 3],
    pub rain: f32,
}

pub const PRESET_NAMES: [&str; 7] = [
    "sunny",
    "deep-clear",
    "overcast",
    "mist",
    "sunset",
    "moonlight",
    "rain",
];

const SHADOW: [f32; 3] = [11.0 / 255.0, 33.0 / 255.0, 30.0 / 255.0];

const SUNNY_BED: BedParams = BedParams {
    deep: [0.486, 0.718, 0.631],
    shallow: [0.145, 0.395, 0.255],
    vertical_tone: 0.8,
    edge_darkening: 0.57,
};

const SUNNY_WATER: WaterParams = WaterParams {
    tint: [0.96, 1.02, 1.0],
    large: [0.022, 0.068, 0.047],
    large_core: [0.052, 0.155, 0.108],
    large_cell: 908.0,
    large_opacity: 0.99,
    large_speed: 1.0,
    secondary: [0.022, 0.068, 0.047],
    secondary_core: [0.052, 0.155, 0.108],
    secondary_cell: 10.0,
    secondary_opacity: 0.15,
    secondary_speed: 1.0,
    detail: [0.01, 0.034, 0.023],
    detail_core: [0.028, 0.09, 0.061],
    detail_cell: 20.0,
    detail_opacity: 0.9,
    detail_speed: 1.0,
};

pub fn preset(name: &str) -> Option<WeatherParams> {
    let mut params = WeatherParams {
        tint: [1.0, 1.0, 1.0],
        brightness: 1.0,
        contrast: 1.0,
        saturation: 1.0,
        vignette: 0.0,
        cloud_strength: 0.0,
        light_color: [1.0, 0.94, 0.72],
        light_strength: 0.0,
        light_direction: [-0.58, 0.82],
        bed: SUNNY_BED,
        water: SUNNY_WATER,
        shadow_color: SHADOW,
        rain: 0.0,
    };

    match name {
        "sunny" => {}
        "deep-clear" => {
            params.bed = BedParams {
                deep: [0.09, 0.15, 0.18],
                shallow: [0.05, 0.02, 0.02],
                vertical_tone: 0.99,
                edge_darkening: 0.57,
            };
            params.water = WaterParams {
                large_core: [0.03744, 0.2325, 0.135],
                large_opacity: 0.59,
                secondary_opacity: 0.1,
                detail_opacity: 0.2,
                ..SUNNY_WATER
            };
        }
        "overcast" => {
            params.tint = [0.87, 0.96, 1.02];
            params.brightness = 0.86;
            params.contrast = 0.9;
            params.saturation = 0.78;
            params.vignette = 0.1;
            params.cloud_strength = 0.62;
            params.light_color = [0.72, 0.84, 0.9];
            params.light_strength = 0.035;
            params.light_direction = [0.42, 0.9];
            params.water = WaterParams {
                large: [0.01452, 0.05848, 0.04935],
                large_core: [0.03224, 0.1364, 0.11664],
                large_opacity: 0.693,
                large_speed: 0.54,
                secondary: [0.01276, 0.05576, 0.05264],
                secondary_core: [0.02808, 0.1333, 0.12528],
                secondary_opacity: 0.087,
                secondary_speed: 0.42,
                detail: [0.007, 0.03128, 0.02484],
                detail_core: [0.01848, 0.0864, 0.06832],
                detail_opacity: 0.495,
                detail_speed: 0.7,
                ..SUNNY_WATER
            };
        }
        "mist" => {
            params.tint = [0.76, 0.9, 0.88];
            params.brightness = 0.8;
            params.contrast = 0.86;
            params.saturation = 0.68;
            params.vignette = 0.14;
            params.cloud_strength = 0.82;
            params.light_color = [0.78, 0.93, 0.88];
            params.light_strength = 0.025;
            params.light_direction = [0.32, 0.95];
            params.water = WaterParams {
                large: [0.01276, 0.06256, 0.03666],
                large_core: [0.02912, 0.1364, 0.081],
                large_cell: 1089.6,
                large_opacity: 0.4356,
                large_speed: 0.24,
                secondary: [0.01144, 0.05848, 0.03572],
                secondary_core: [0.026, 0.1271, 0.07776],
                secondary_cell: 17.0,
                secondary_opacity: 0.054,
                secondary_speed: 0.18,
                detail: [0.0062, 0.03332, 0.01978],
                detail_core: [0.0168, 0.0846, 0.05002],
                detail_cell: 26.0,
                detail_opacity: 0.288,
                detail_speed: 0.3,
                ..SUNNY_WATER
            };
        }
        "sunset" => {
            params.tint = [1.08, 0.86, 0.7];
            params.brightness = 0.93;
            params.contrast = 1.06;
            params.saturation = 1.1;
            params.vignette = 0.2;
            params.cloud_strength = 0.16;
            params.light_color = [1.0, 0.42, 0.15];
            params.light_strength = 0.2;
            params.light_direction = [-0.7, 0.72];
            params.water = WaterParams {
                large: [0.03124, 0.05168, 0.0235],
                large_core: [0.06968, 0.1054, 0.0432],
                large_opacity: 0.9108,
                large_speed: 0.72,
                secondary: [0.0264, 0.04216, 0.02162],
                secondary_core: [0.06656, 0.1023, 0.0432],
                secondary_opacity: 0.099,
                secondary_speed: 0.5,
                detail: [0.0128, 0.02856, 0.01334],
                detail_core: [0.03808, 0.0774, 0.0305],
                detail_opacity: 0.468,
                detail_speed: 0.86,
                ..SUNNY_WATER
            };
        }
        "moonlight" => {
            params.tint = [0.5, 0.7, 1.04];
            params.brightness = 0.6;
            params.contrast = 1.08;
            params.saturation = 0.76;
            params.vignette = 0.42;
            params.cloud_strength = 0.3;
            params.light_color = [0.46, 0.68, 1.0];
            params.light_strength = 0.14;
            params.light_direction = [0.68, 0.74];
            params.water = WaterParams {
                large: [0.01144, 0.05304, 0.0705],
                large_core: [0.02496, 0.1147, 0.17496],
                large_opacity: 0.7128,
                large_speed: 0.34,
                secondary: [0.01012, 0.0476, 0.07426],
                secondary_core: [0.02184, 0.1054, 0.18144],
                secondary_opacity: 0.075,
                secondary_speed: 0.24,
                detail: [0.0058, 0.02924, 0.03266],
                detail_core: [0.01512, 0.081, 0.09272],
                detail_opacity: 0.702,
                detail_speed: 0.44,
                ..SUNNY_WATER
            };
        }
        "rain" => {
            params.tint = [0.67, 0.86, 0.96];
            params.brightness = 0.72;
            params.contrast = 0.94;
            params.saturation = 0.72;
            params.vignette = 0.28;
            params.cloud_strength = 0.78;
            params.light_color = [0.54, 0.75, 0.86];
            params.light_strength = 0.045;
            params.light_direction = [0.36, 0.93];
            params.rain = 10.0;
            params.water = WaterParams {
                large: [0.01188, 0.0612, 0.06298],
                large_core: [0.02496, 0.1426, 0.15552],
                large_opacity: 1.1088,
                large_speed: 1.42,
                secondary: [0.01056, 0.05712, 0.06674],
                secondary_core: [0.02288, 0.1364, 0.16416],
                secondary_opacity: 0.138,
                secondary_speed: 1.18,
                detail: [0.0062, 0.034, 0.02944],
                detail_core: [0.01624, 0.0936, 0.08296],
                detail_opacity: 1.152,
                detail_speed: 1.72,
                ..SUNNY_WATER
            };
        }
        _ => return None,
    }
    Some(params)
}

pub fn default_params() -> WeatherParams {
    preset("sunny").expect("sunny preset")
}

pub struct Weather {
    current: WeatherParams,
    target: WeatherParams,
    name: String,
}

impl Weather {
    pub fn new() -> Self {
        let sunny = default_params();
        Self {
            current: sunny,
            target: sunny,
            name: "sunny".to_string(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set(&mut self, name: &str) -> Result<(), String> {
        match preset(name) {
            Some(params) => {
                self.target = params;
                self.name = name.to_string();
                Ok(())
            }
            None => Err(format!(
                "unknown ambience '{}'; choose one of: {}",
                name,
                PRESET_NAMES.join(", ")
            )),
        }
    }

    pub fn update(&mut self, delta: f32) {
        let blend = 1.0 - (-delta.clamp(0.0, 0.1) * 2.25).exp();
        blend_params(&mut self.current, &self.target, blend);
    }

    pub fn current(&self) -> &WeatherParams {
        &self.current
    }
}

fn blend_color(current: &mut [f32; 3], target: &[f32; 3], blend: f32) {
    for (slot, goal) in current.iter_mut().zip(target.iter()) {
        *slot += (*goal - *slot) * blend;
    }
}

fn blend_scalar(current: &mut f32, target: f32, blend: f32) {
    *current += (target - *current) * blend;
}

fn blend_bed(current: &mut BedParams, target: &BedParams, blend: f32) {
    blend_color(&mut current.deep, &target.deep, blend);
    blend_color(&mut current.shallow, &target.shallow, blend);
    blend_scalar(&mut current.vertical_tone, target.vertical_tone, blend);
    blend_scalar(&mut current.edge_darkening, target.edge_darkening, blend);
}

#[allow(clippy::too_many_lines)]
fn blend_water(current: &mut WaterParams, target: &WaterParams, blend: f32) {
    blend_color(&mut current.tint, &target.tint, blend);
    blend_color(&mut current.large, &target.large, blend);
    blend_color(&mut current.large_core, &target.large_core, blend);
    blend_scalar(&mut current.large_cell, target.large_cell, blend);
    blend_scalar(&mut current.large_opacity, target.large_opacity, blend);
    blend_scalar(&mut current.large_speed, target.large_speed, blend);
    blend_color(&mut current.secondary, &target.secondary, blend);
    blend_color(&mut current.secondary_core, &target.secondary_core, blend);
    blend_scalar(&mut current.secondary_cell, target.secondary_cell, blend);
    blend_scalar(&mut current.secondary_opacity, target.secondary_opacity, blend);
    blend_scalar(&mut current.secondary_speed, target.secondary_speed, blend);
    blend_color(&mut current.detail, &target.detail, blend);
    blend_color(&mut current.detail_core, &target.detail_core, blend);
    blend_scalar(&mut current.detail_cell, target.detail_cell, blend);
    blend_scalar(&mut current.detail_opacity, target.detail_opacity, blend);
    blend_scalar(&mut current.detail_speed, target.detail_speed, blend);
}

fn blend_params(current: &mut WeatherParams, target: &WeatherParams, blend: f32) {
    blend_color(&mut current.tint, &target.tint, blend);
    blend_scalar(&mut current.brightness, target.brightness, blend);
    blend_scalar(&mut current.contrast, target.contrast, blend);
    blend_scalar(&mut current.saturation, target.saturation, blend);
    blend_scalar(&mut current.vignette, target.vignette, blend);
    blend_scalar(&mut current.cloud_strength, target.cloud_strength, blend);
    blend_color(&mut current.light_color, &target.light_color, blend);
    blend_scalar(&mut current.light_strength, target.light_strength, blend);
    for (slot, goal) in current
        .light_direction
        .iter_mut()
        .zip(target.light_direction.iter())
    {
        *slot += (*goal - *slot) * blend;
    }
    blend_bed(&mut current.bed, &target.bed, blend);
    blend_water(&mut current.water, &target.water, blend);
    blend_color(&mut current.shadow_color, &target.shadow_color, blend);
    blend_scalar(&mut current.rain, target.rain, blend);
}
