//! Unix-socket control for the running wallpaper.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::time::Duration;

pub fn socket_path() -> PathBuf {
    let dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(dir).join("nagomi-native.sock")
}

pub fn listen() -> std::io::Result<UnixListener> {
    let path = socket_path();
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path)?;
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    listener.set_nonblocking(true)?;
    Ok(listener)
}

pub fn help() -> String {
    format!(
        "\
usage: nagomi <command> [args]

  fish [1-48]        show or set the number of koi
  weather            show the current ambience and the available names
  weather <name>     switch ambience: {}
  ambience <name>    alias for weather
  scale [1-4]        show or set render resolution; 1 = 480x270 pixel art, 4 = native
  leaves [0-19]      show or set the number of lotus leaves
  flowers [0-4]      show or set the number of lotus flowers
  duckweed [0-8]     show or set the number of duckweed patches
  butterflies [0-4]  show or set the number of butterflies
  scatter            startle the koi
  status             fish count, ambience, pid, uptime
  help               this text

Run with no command to start the wallpaper (normally systemd does this).
Fish count and ambience are remembered across restarts.
",
        crate::weather::PRESET_NAMES.join(", ")
    )
}

pub fn run_client(args: &[String]) -> i32 {
    let line = args.join(" ");
    match send(&line) {
        Ok(response) => {
            print!("{response}");
            0
        }
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

fn send(line: &str) -> Result<String, String> {
    let mut stream = UnixStream::connect(socket_path())
        .map_err(|_| "nagomi: wallpaper is not running".to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("nagomi: {error}"))?;
    stream
        .write_all(line.as_bytes())
        .and_then(|_| stream.write_all(b"\n"))
        .map_err(|error| format!("nagomi: {error}"))?;
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader
        .read_to_string(&mut response)
        .map_err(|error| format!("nagomi: {error}"))?;
    Ok(response)
}

pub fn read_request(stream: &UnixStream) -> Option<String> {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(300)));
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(line.trim().to_string()),
    }
}
