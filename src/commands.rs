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
                Ok(()) => {
                    scene.weather.set_cycle(false);
                    format!("ambience {name}\n")
                }
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
        "cycle" => match words.next() {
            None => format!(
                "cycle {}\n",
                if scene.weather.cycle_enabled() { "on" } else { "off" }
            ),
            Some("on") => {
                scene.weather.set_cycle(true);
                use chrono::Timelike;
                let local = chrono::Local::now();
                let hour = local.hour() as f32 + local.minute() as f32 / 60.0;
                scene.weather.apply_cycle(hour);
                "cycle on\n".to_string()
            }
            Some("off") => {
                scene.weather.set_cycle(false);
                "cycle off\n".to_string()
            }
            Some(_) => "nagomi: cycle takes on or off\n".to_string(),
        },
        "autostart" => match words.next() {
            None => autostart(None),
            Some("on") => autostart(Some(true)),
            Some("off") => autostart(Some(false)),
            Some(_) => "nagomi: autostart takes on or off\n".to_string(),
        },
        "scatter" => {
            scene.school.scatter();
            "ok\n".to_string()
        }
        "status" => format!(
            "fish {}\nambience {}\ncycle {}\nscale {}\nleaves {} flowers {} duckweed {} butterflies {}\npid {}\nuptime {:.0}s\n",
            scene.school.count,
            scene.weather.name(),
            if scene.weather.cycle_enabled() { "on" } else { "off" },
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
        None | Some("status" | "help" | "--help" | "-h" | "scatter" | "autostart")
    ) {
        SavedState::save(scene, renderer.scale());
    }
    reply
}

#[cfg(windows)]
fn autostart(on: Option<bool>) -> String {
    use std::process::Command;

    const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    match on {
        None => match Command::new("reg").args(["query", KEY, "/v", "nagomi"]).output() {
            Ok(output) if output.status.success() => "autostart on\n".to_string(),
            _ => "autostart off\n".to_string(),
        },
        Some(true) => {
            let Ok(exe) = std::env::current_exe() else {
                return "nagomi: cannot find the executable\n".to_string();
            };
            match Command::new("reg")
                .args([
                    "add",
                    KEY,
                    "/v",
                    "nagomi",
                    "/t",
                    "REG_SZ",
                    "/d",
                    &exe.display().to_string(),
                    "/f",
                ])
                .output()
            {
                Ok(output) if output.status.success() => "autostart on\n".to_string(),
                _ => "nagomi: could not write the Run key\n".to_string(),
            }
        }
        Some(false) => {
            match Command::new("reg")
                .args(["delete", KEY, "/v", "nagomi", "/f"])
                .output()
            {
                Ok(output) if output.status.success() => "autostart off\n".to_string(),
                _ => "nagomi: autostart is already off\n".to_string(),
            }
        }
    }
}

#[cfg(not(windows))]
fn autostart(_on: Option<bool>) -> String {
    "nagomi: on Linux autostart is the systemd unit: systemctl --user enable --now nagomi-wallpaper.service\n"
        .to_string()
}
