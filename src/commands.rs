//! CLI request handling, shared by both front ends.

use std::time::Instant;

use crate::control;
use crate::gpu;
use crate::scene::Scene;
use crate::sim;
use crate::state::SavedState;
use crate::weather;

pub fn handle(
    request: &str,
    scene: &mut Scene,
    renderer: &mut gpu::Renderer,
    started: Instant,
) -> String {
    let mut words = request.split_whitespace();
    let reply = match words.next().unwrap_or("") {
        "" => return String::new(),
        "fish" => match words.next() {
            None => format!("fish {}\n", scene.school.count),
            Some(value) => match value.parse::<usize>() {
                Ok(count) => {
                    let count = count.clamp(1, sim::MAX_FISH);
                    scene.school.count = count;
                    format!("fish {count}\n")
                }
                Err(_) => format!(
                    "nagomi: fish needs a number between 1 and {}\n",
                    sim::MAX_FISH
                ),
            },
        },
        "weather" | "ambience" => match words.next() {
            None => format!(
                "ambience {}\navailable: {}\n",
                scene.weather.name(),
                weather::PRESET_NAMES.join(", ")
            ),
            Some(name) => match scene.weather.set(name) {
                Ok(()) => format!("ambience {name}\n"),
                Err(error) => format!("nagomi: {error}\n"),
            },
        },
        "scale" => match words.next() {
            None => format!("scale {}\n", renderer.scale()),
            Some(value) => match value.parse::<u32>() {
                Ok(scale) => match renderer.set_scale(scale) {
                    Ok(()) => format!("scale {scale}\n"),
                    Err(error) => format!("nagomi: {error}\n"),
                },
                Err(_) => "nagomi: scale needs a number between 1 and 4\n".to_string(),
            },
        },
        "leaves" => match words.next() {
            None => format!("leaves {}\n", scene.lotus.visible_leaves()),
            Some(value) => match value.parse::<usize>() {
                Ok(count) => {
                    let count = scene.lotus.set_visible_leaves(count);
                    format!("leaves {count}\n")
                }
                Err(_) => "nagomi: leaves needs a number\n".to_string(),
            },
        },
        "flowers" => match words.next() {
            None => format!("flowers {}\n", scene.lotus.visible_flowers()),
            Some(value) => match value.parse::<usize>() {
                Ok(count) => {
                    let count = scene.lotus.set_visible_flowers(count);
                    format!("flowers {count}\n")
                }
                Err(_) => "nagomi: flowers needs a number\n".to_string(),
            },
        },
        "duckweed" => match words.next() {
            None => format!("duckweed {}\n", scene.duckweed.visible_patches()),
            Some(value) => match value.parse::<usize>() {
                Ok(count) => {
                    let count = scene.duckweed.set_visible_patches(count);
                    format!("duckweed {count}\n")
                }
                Err(_) => "nagomi: duckweed needs a number\n".to_string(),
            },
        },
        "butterflies" => match words.next() {
            None => format!("butterflies {}\n", scene.butterflies.visible()),
            Some(value) => match value.parse::<usize>() {
                Ok(count) => {
                    let count = scene.butterflies.set_visible(count);
                    format!("butterflies {count}\n")
                }
                Err(_) => "nagomi: butterflies needs a number\n".to_string(),
            },
        },
        "scatter" => {
            scene.school.scatter();
            "ok\n".to_string()
        }
        "status" => format!(
            "fish {}\nambience {}\nscale {}\nleaves {} flowers {} duckweed {} butterflies {}\npid {}\nuptime {:.0}s\n",
            scene.school.count,
            scene.weather.name(),
            renderer.scale(),
            scene.lotus.visible_leaves(),
            scene.lotus.visible_flowers(),
            scene.duckweed.visible_patches(),
            scene.butterflies.visible(),
            std::process::id(),
            started.elapsed().as_secs_f32()
        ),
        "help" | "--help" | "-h" => control::help(),
        other => format!("nagomi: unknown command '{other}'\n{}", control::help()),
    };

    // Persist after anything that mutates state; the reply tells whether it did.
    if !reply.starts_with("nagomi:") && !matches!(
        request.split_whitespace().next(),
        None | Some("status" | "help" | "--help" | "-h" | "scatter")
    ) {
        SavedState::save(scene, renderer.scale());
    }
    reply
}
