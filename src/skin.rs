//! Local, user-supplied sprite skin. The application ships no third-party art.

use std::path::PathBuf;

pub struct DecodedSprite {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub fn skins_root() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
        });
    base.join("nagomi/skins")
}

pub fn list() -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(skins_root()) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
        .collect();
    names.sort();
    names
}

pub struct Skin {
    pub koi: Vec<Option<DecodedSprite>>,
    pub leaf: Vec<Option<DecodedSprite>>,
}

impl Skin {
    pub fn empty() -> Self {
        Self {
            koi: (0..6).map(|_| None).collect(),
            leaf: (0..2).map(|_| None).collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.koi.iter().all(Option::is_none) && self.leaf.iter().all(Option::is_none)
    }
}

pub fn load(name: &str) -> Skin {
    let dir = skins_root().join(name);
    let mut skin = Skin::empty();
    for (index, slot) in skin.koi.iter_mut().enumerate() {
        *slot = decode(&dir.join(format!("koi{}.png", index + 1)));
    }
    for (index, slot) in skin.leaf.iter_mut().enumerate() {
        *slot = decode(&dir.join(format!("leaf{}.png", index + 1)));
    }
    skin
}

fn decode(path: &std::path::Path) -> Option<DecodedSprite> {
    let image = image::open(path).ok()?.to_rgba8();
    Some(DecodedSprite {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    })
}
