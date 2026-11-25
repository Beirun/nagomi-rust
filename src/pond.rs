//! Pond variants: the same simulation wearing four different palettes.

use crate::weather::WeatherParams;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Pond {
    #[default]
    Classic,
    Grove,
    Maple,
    Lagoon,
}

pub const POND_NAMES: [&str; 4] = ["classic", "grove", "maple", "lagoon"];

#[derive(Clone, Copy)]
pub struct LeafColors {
    pub base: [f32; 3],
    pub light: [f32; 3],
    pub shade: [f32; 3],
    pub vein: [f32; 3],
    pub center: [f32; 3],
}

#[derive(Clone, Copy)]
pub struct WeedColors {
    pub base: [f32; 3],
    pub light: [f32; 3],
    pub shade: [f32; 3],
    pub center: [f32; 3],
}

const fn rgb(hex: u32) -> [f32; 3] {
    [
        ((hex >> 16) & 0xff) as f32 / 255.0,
        ((hex >> 8) & 0xff) as f32 / 255.0,
        (hex & 0xff) as f32 / 255.0,
    ]
}

const fn leaf(base: u32, light: u32, shade: u32, vein: u32, center: u32) -> LeafColors {
    LeafColors {
        base: rgb(base),
        light: rgb(light),
        shade: rgb(shade),
        vein: rgb(vein),
        center: rgb(center),
    }
}

const fn weed(base: u32, light: u32, shade: u32, center: u32) -> WeedColors {
    WeedColors {
        base: rgb(base),
        light: rgb(light),
        shade: rgb(shade),
        center: rgb(center),
    }
}

const CLASSIC_LEAVES: [LeafColors; 2] = [
    leaf(0x1d5630, 0x2e673b, 0x113322, 0x0d291d, 0x143d25),
    leaf(0x184629, 0x265b33, 0x0d291b, 0x0a2216, 0x113420),
];

const GROVE_LEAVES: [LeafColors; 2] = [
    leaf(0x1b4f38, 0x2a644a, 0x123c2a, 0x2c6a48, 0x164232),
    leaf(0x17452f, 0x245a3f, 0x0f3523, 0x25603f, 0x123b2a),
];

const MAPLE_LEAVES: [LeafColors; 2] = [
    leaf(0x4a5a22, 0x66742c, 0x36401a, 0x2a3314, 0x41501e),
    leaf(0x565021, 0x706a2b, 0x3e3a17, 0x323010, 0x4b461c),
];

const LAGOON_LEAVES: [LeafColors; 2] = [
    leaf(0x55975f, 0x74b276, 0x447a4d, 0xdcf0d6, 0x4f8c58),
    leaf(0x5da267, 0x7cbb7e, 0x4a8252, 0xe2f4dc, 0x569660),
];

const CLASSIC_WEED: [WeedColors; 2] = [
    weed(0x6fc94f, 0x9be66c, 0x45963e, 0xc3ee75),
    weed(0x83d35b, 0xb1ed76, 0x549e43, 0xd0f28a),
];

const GROVE_WEED: [WeedColors; 2] = [
    weed(0x55a83c, 0x7dc455, 0x357a30, 0xa5d964),
    weed(0x619e3f, 0x8cbb5c, 0x3f7a33, 0xb4d876),
];

const MAPLE_WEED: [WeedColors; 2] = [
    weed(0xb4551c, 0xd97a2b, 0x8a3a12, 0xe29a45),
    weed(0xa33d16, 0xcc6426, 0x7c2e0e, 0xd98a3f),
];

const LAGOON_WEED: [WeedColors; 2] = [
    weed(0x7cb86a, 0xa2d187, 0x5c9750, 0xb9e39a),
    weed(0x87c273, 0xaad894, 0x649d56, 0xc0e8a3),
];

impl Pond {
    pub fn name(self) -> &'static str {
        match self {
            Pond::Classic => "classic",
            Pond::Grove => "grove",
            Pond::Maple => "maple",
            Pond::Lagoon => "lagoon",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "classic" => Some(Pond::Classic),
            "grove" => Some(Pond::Grove),
            "maple" => Some(Pond::Maple),
            "lagoon" => Some(Pond::Lagoon),
            _ => None,
        }
    }

    pub fn leaves(self) -> [LeafColors; 2] {
        match self {
            Pond::Classic => CLASSIC_LEAVES,
            Pond::Grove => GROVE_LEAVES,
            Pond::Maple => MAPLE_LEAVES,
            Pond::Lagoon => LAGOON_LEAVES,
        }
    }

    pub fn weed(self) -> [WeedColors; 2] {
        match self {
            Pond::Classic => CLASSIC_WEED,
            Pond::Grove => GROVE_WEED,
            Pond::Maple => MAPLE_WEED,
            Pond::Lagoon => LAGOON_WEED,
        }
    }

    pub fn leaf_scale(self) -> f32 {
        match self {
            Pond::Classic => 1.18,
            Pond::Grove => 1.05,
            Pond::Maple => 1.12,
            Pond::Lagoon => 1.7,
        }
    }

    /// Radial veins per pad, and how far toward the rim they reach.
    pub fn vein_count(self) -> usize {
        match self {
            Pond::Classic => 5,
            Pond::Grove => 12,
            Pond::Maple => 5,
            Pond::Lagoon => 20,
        }
    }

    pub fn vein_reach(self) -> f32 {
        match self {
            Pond::Classic => 0.68,
            Pond::Grove => 0.86,
            Pond::Maple => 0.68,
            Pond::Lagoon => 0.9,
        }
    }

    pub fn has_pad_rim(self) -> bool {
        matches!(self, Pond::Lagoon)
    }

    pub fn has_rocks(self) -> bool {
        !matches!(self, Pond::Classic)
    }

    pub fn has_maple_leaves(self) -> bool {
        matches!(self, Pond::Maple)
    }

    /// Default (leaves, flowers, duckweed) so each pond has its own density.
    pub fn defaults(self) -> (usize, usize, usize) {
        match self {
            Pond::Classic => (15, 4, 8),
            Pond::Grove => (32, 9, 10),
            Pond::Maple => (12, 0, 12),
            Pond::Lagoon => (10, 3, 4),
        }
    }

    /// Bed and water-layer multipliers against the weather preset. Only the
    /// background is graded, so the koi keep their own colors.
    pub fn grade(self) -> ([f32; 3], [f32; 3]) {
        match self {
            Pond::Classic => ([1.0; 3], [1.0; 3]),
            Pond::Grove => ([0.82, 0.95, 0.9], [0.72, 0.93, 0.86]),
            Pond::Maple => ([0.32, 0.42, 0.33], [0.45, 0.62, 0.48]),
            Pond::Lagoon => ([1.15, 1.5, 1.65], [0.28, 0.55, 1.65]),
        }
    }
}

pub fn graded(weather: &WeatherParams, pond: Pond) -> WeatherParams {
    let (bed, water) = pond.grade();
    let mut out = *weather;
    let mul = |a: [f32; 3], b: [f32; 3]| [a[0] * b[0], a[1] * b[1], a[2] * b[2]];

    out.bed.deep = mul(out.bed.deep, bed);
    out.bed.shallow = mul(out.bed.shallow, bed);
    for color in [
        &mut out.water.large,
        &mut out.water.large_core,
        &mut out.water.secondary,
        &mut out.water.secondary_core,
        &mut out.water.detail,
        &mut out.water.detail_core,
    ] {
        *color = mul(*color, water);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for pond in [Pond::Classic, Pond::Grove, Pond::Maple, Pond::Lagoon] {
            assert_eq!(Pond::from_name(pond.name()), Some(pond));
        }
        assert_eq!(Pond::from_name("swamp"), None);
    }

    #[test]
    fn classic_grade_is_identity() {
        let weather = crate::weather::preset("sunny").unwrap();
        let graded = graded(&weather, Pond::Classic);
        assert_eq!(graded.bed.deep, weather.bed.deep);
        assert_eq!(graded.water.large_core, weather.water.large_core);
        assert_eq!(graded.tint, weather.tint);
    }
}
