//! Persisted CLI state, shared by the Linux and Windows front ends.

use std::path::PathBuf;

use crate::scene::Scene;
use crate::sim;

#[derive(Default)]
pub struct SavedState {
    pub fish: Option<usize>,
    pub ambience: Option<String>,
    pub scale: Option<u32>,
    pub leaves: Option<usize>,
    pub flowers: Option<usize>,
    pub duckweed: Option<usize>,
    pub butterflies: Option<usize>,
}

fn base_dir() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                    .join(".local/state")
            })
    }
}

pub fn state_path() -> PathBuf {
    base_dir().join("nagomi/state")
}

impl SavedState {
    pub fn load() -> Self {
        let mut saved = SavedState::default();
        let Ok(text) = std::fs::read_to_string(state_path()) else {
            return saved;
        };
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match key {
                "fish" => saved.fish = value.parse().ok(),
                "ambience" => saved.ambience = Some(value.to_string()),
                "scale" => saved.scale = value.parse().ok(),
                "leaves" => saved.leaves = value.parse().ok(),
                "flowers" => saved.flowers = value.parse().ok(),
                "duckweed" => saved.duckweed = value.parse().ok(),
                "butterflies" => saved.butterflies = value.parse().ok(),
                _ => {}
            }
        }
        saved
    }

    pub fn apply(&self, scene: &mut Scene) {
        if let Some(name) = &self.ambience {
            let _ = scene.weather.set(name);
        }
        if let Some(count) = self.fish {
            scene.school.count = count.clamp(1, sim::MAX_FISH);
        }
        if let Some(count) = self.leaves {
            scene.lotus.set_visible_leaves(count);
        }
        if let Some(count) = self.flowers {
            scene.lotus.set_visible_flowers(count);
        }
        if let Some(count) = self.duckweed {
            scene.duckweed.set_visible_patches(count);
        }
        if let Some(count) = self.butterflies {
            scene.butterflies.set_visible(count);
        }
    }

    pub fn save(scene: &Scene, scale: u32) {
        let contents = format!(
            "fish={}\nambience={}\nscale={}\nleaves={}\nflowers={}\nduckweed={}\nbutterflies={}\n",
            scene.school.count,
            scene.weather.name(),
            scale,
            scene.lotus.visible_leaves(),
            scene.lotus.visible_flowers(),
            scene.duckweed.visible_patches(),
            scene.butterflies.visible(),
        );
        let path = state_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, contents);
    }
}
