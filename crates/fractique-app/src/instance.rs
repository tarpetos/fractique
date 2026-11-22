use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, mpsc};

use eframe::egui;
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, Listener, ListenerOptions, Name, Stream, prelude::*,
};

static FIRST: Mutex<Option<Listener>> = Mutex::new(None);
static LEAVING: AtomicBool = AtomicBool::new(false);
static LEFT: Mutex<Option<mpsc::Receiver<()>>> = Mutex::new(None);

pub const AFTER_UPDATE: &str = "FRACTIQUE_AFTER_UPDATE";

// A second start: the file it was given, and the token with which Wayland lets the running app's window come to the
// front, which the system hands to the program it starts.
pub struct Arrival {
    pub path: Option<PathBuf>,
    pub token: Option<String>,
}

fn label() -> String {
    // A test has a name of its own: the owner's app may be running beside it.
    if cfg!(test) {
        return format!("fractique-test-{}", std::process::id());
    }
    let user = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_default();
    let user: String = user.chars().filter(|c| c.is_alphanumeric()).collect();
    format!("fractique-instance-{user}")
}

fn socket_file() -> PathBuf {
    std::env::temp_dir().join(format!("{}.sock", label()))
}

fn name() -> std::io::Result<Name<'static>> {
    if GenericNamespaced::is_supported() {
        label().to_ns_name::<GenericNamespaced>()
    } else {
        socket_file().to_fs_name::<GenericFilePath>()
    }
}

fn hand_over(open: Option<&Path>) -> std::io::Result<()> {
    let mut stream = BufReader::new(Stream::connect(name()?)?);
    let line = open.map(|path| path.to_string_lossy().into_owned()).unwrap_or_default();
    stream.get_mut().write_all(format!("{line}\n").as_bytes())?;
    stream.read_line(&mut String::new())?;
    // After the answer, so that an app before 1.0.2, which reads one line, is not kept waiting.
    if let Some(token) = std::env::var("XDG_ACTIVATION_TOKEN")
        .ok()
        .filter(|token| !token.is_empty())
    {
        let _ = stream.get_mut().write_all(format!("{token}\n").as_bytes());
    }
    Ok(())
}

fn listen() -> std::io::Result<()> {
    let listener = ListenerOptions::new().name(name()?).create_sync()?;
    if let Ok(mut first) = FIRST.lock() {
        *first = Some(listener);
    }
    Ok(())
}

fn freed() -> bool {
    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if listen().is_ok() {
            return true;
        }
    }
    false
}

pub fn first(open: Option<&Path>) -> bool {
    // The app an update started in its own place: the one that started it is still closing and would be handed
    // the start, so this one waits for its name instead.
    if std::env::var_os(AFTER_UPDATE).is_some() {
        for _ in 0..100 {
            if listen().is_ok() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    for _ in 0..2 {
        if name().is_err() {
            return true;
        }
        match listen() {
            Ok(()) => return true,
            // An app before 1.0.1 that updated itself starts the new one while it still listens, and closes: its
            // name is free moments later, and this start, which has no file to hand over, is then the app.
            Err(_) if hand_over(open).is_ok() => return open.is_none() && freed(),
            // A socket file whose owner is gone: nobody answers, and the name stays taken until the file is removed.
            Err(_) => {
                let _ = std::fs::remove_file(socket_file());
            }
        }
    }
    true
}

pub fn watch(ctx: &egui::Context) -> Option<mpsc::Receiver<Arrival>> {
    let listener = FIRST.lock().ok()?.take()?;
    let (arrived, arrivals) = mpsc::channel();
    let ctx = ctx.clone();
    let (leave, left) = mpsc::channel();
    if let Ok(mut kept) = LEFT.lock() {
        *kept = Some(left);
    }
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            if LEAVING.load(Ordering::Relaxed) {
                break;
            }
            let Ok(stream) = stream else {
                continue;
            };
            let mut stream = BufReader::new(stream);
            let mut line = String::new();
            if stream.read_line(&mut line).is_err() {
                continue;
            }
            let _ = stream.get_mut().write_all(b"\n");
            let path = line.trim_end_matches(['\r', '\n']);
            let path = (!path.is_empty()).then(|| PathBuf::from(path));
            // The token follows the answer; a start before 1.0.2 sends none and closes.
            let mut token = String::new();
            let _ = stream.read_line(&mut token);
            let token = token.trim();
            let token = (!token.is_empty()).then(|| token.to_owned());
            if arrived.send(Arrival { path, token }).is_err() {
                break;
            }
            ctx.request_repaint();
        }
        drop(listener);
        let _ = leave.send(());
    });
    Some(arrivals)
}

// Gives the name up before this app starts another in its own place: that one is then the first whatever its
// version, also one that does not know to wait for the name.
pub fn release() {
    if let Ok(mut first) = FIRST.lock() {
        *first = None;
    }
    LEAVING.store(true, Ordering::Relaxed);
    let Some(left) = LEFT.lock().ok().and_then(|mut left| left.take()) else {
        return;
    };
    // The thread that listens waits for a caller: this one wakes it.
    if let Ok(name) = name() {
        let _ = Stream::connect(name);
    }
    let _ = left.recv_timeout(std::time::Duration::from_secs(2));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_is_free_once_the_app_gives_it_up() {
        assert!(listen().is_ok());
        let arrivals = watch(&egui::Context::default());
        assert!(arrivals.is_some());
        // The name is taken while the app listens: a second start would be handed over.
        assert!(listen().is_err());
        assert!(hand_over(None).is_ok());
        let arrival = arrivals
            .as_ref()
            .and_then(|arrivals| arrivals.recv_timeout(std::time::Duration::from_secs(2)).ok());
        assert!(arrival.is_some_and(|arrival| arrival.path.is_none()));
        release();
        // Given up, it is free for the app started in this one's place, whatever its version.
        assert!(listen().is_ok());
        if let Ok(mut first) = FIRST.lock() {
            *first = None;
        }
    }
}
