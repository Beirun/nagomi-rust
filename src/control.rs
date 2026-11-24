//! Control transport: a Unix socket on Linux, loopback TCP on Windows.

pub fn help() -> String {
    format!(
        "\
usage: nagomi <command> [args]

  fish [1-48]        show or set the number of koi
  weather            show the current ambience and the available names
  weather <name>     switch ambience: {}
  ambience <name>    alias for weather
  cycle [on|off]     follow the clock: dawn, day, golden hour, dusk, night
  scale [1-4]        show or set render resolution; 1 = 480x270 pixel art, 4 = native
  leaves [0-32]      show or set the number of lotus leaves
  flowers [0-16]     show or set the number of lotus flowers
  duckweed [0-16]    show or set the number of duckweed patches
  butterflies [0-16] show or set the number of butterflies
  autostart [on|off] show or set start-with-session (Windows; Linux uses systemd)
  scatter            startle the koi
  status             fish count, ambience, scale, scene counts, pid, uptime
  help               this text

Run with no command to start the wallpaper (normally it starts with the session).
Fish count and ambience are remembered across restarts.
",
        crate::weather::PRESET_NAMES.join(", ")
    )
}

#[cfg(unix)]
pub use unix_impl::{listen, read_request, run_client, socket_path};

#[cfg(windows)]
pub use windows_impl::{run_client, spawn_server, Request, PORT};

#[cfg(unix)]
mod unix_impl {
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
}

#[cfg(windows)]
mod windows_impl {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    pub const PORT: u16 = 7391;

    pub struct Request {
        pub text: String,
        pub reply: mpsc::Sender<String>,
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
        let mut stream = TcpStream::connect(("127.0.0.1", PORT))
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

    /// Accept commands on loopback and forward them to the window thread.
    pub fn spawn_server() -> Option<mpsc::Receiver<Request>> {
        let listener = TcpListener::bind(("127.0.0.1", PORT)).ok()?;
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(&stream);
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() {
                    continue;
                }
                let (reply_tx, reply_rx) = mpsc::channel();
                let request = Request {
                    text: line.trim().to_string(),
                    reply: reply_tx,
                };
                if sender.send(request).is_err() {
                    break;
                }
                if let Ok(response) = reply_rx.recv_timeout(Duration::from_secs(2)) {
                    let mut writer = &stream;
                    let _ = writer.write_all(response.as_bytes());
                }
            }
        });
        Some(receiver)
    }
}
