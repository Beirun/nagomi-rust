mod butterflies;
mod commands;
mod control;
mod decor;
mod duckweed;
mod fish;
mod gpu;
mod lotus;
mod pond;
mod scene;
mod sim;
mod state;
mod tiny;
mod weather;

#[cfg(unix)]
mod platform_linux;
#[cfg(windows)]
mod platform_windows;

fn main() {
    env_logger::init();

    #[cfg(unix)]
    if let Err(error) = platform_linux::run() {
        eprintln!("nagomi: {error}");
        std::process::exit(1);
    }
    #[cfg(windows)]
    platform_windows::run();
}
