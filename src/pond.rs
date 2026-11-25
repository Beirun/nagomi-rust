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
    leaf(0x1b4f38, 0x2a644a, 0x123c2a, 0x0d2d1f, 0x164232),
    leaf(0x17452f, 0x245a3f, 0x0f3523, 0x0b291a, 0x123b2a),
];

const MAPLE_LEAVES: [LeafColors; 2] = [
    leaf(0x6e3a14, 0x8a4d1c, 0x4a2509, 0x311707, 0x5e3210),
    leaf(0x7c4418, 0x995a22, 0x52280b, 0x381a08, 0x6b3a14),
];

const LAGOON_LEAVES: [LeafColors; 2] = [
    leaf(0x8fbf95, 0xadd4ac, 0x6d9c77, 0x527c5d, 0x7fae86),
    leaf(0x9ac79c, 0xb8dcb6, 0x74a27f, 0x578162, 0x88b891),
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
            Pond::Grove => 1.26,
            Pond::Maple => 1.12,
            Pond::Lagoon => 1.62,
        }
    }

    /// Default (leaves, flowers, duckweed) so each pond has its own density.
    pub fn defaults(self) -> (usize, usize, usize) {
        match self {
            Pond::Classic => (15, 4, 8),
            Pond::Grove => (30, 9, 10),
            Pond::Maple => (16, 0, 12),
            Pond::Lagoon => (10, 3, 4),
        }
    }

    /// Bed and water-layer multipliers against the weather preset. Only the
    /// background is graded, so the koi keep their own colors.
    pub fn grade(self) -> ([f32; 3], [f32; 3]) {
        match self {
            Pond::Classic => ([1.0; 3], [1.0; 3]),
            Pond::Grove => ([0.9, 1.0, 0.97], [0.82, 1.0, 0.94]),
            Pond::Maple => ([0.5, 0.32, 0.16], [0.9, 0.5, 0.24]),
            Pond::Lagoon => ([1.2, 1.1, 1.55], [0.22, 0.42, 2.0]),
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
