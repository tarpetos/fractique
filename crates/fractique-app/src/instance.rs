use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, mpsc};

use eframe::egui;
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, Listener, ListenerOptions, Name, Stream, prelude::*,
};

static FIRST: Mutex<Option<Listener>> = Mutex::new(None);

fn label() -> String {
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
    Ok(())
}

pub fn first(open: Option<&Path>) -> bool {
    for _ in 0..2 {
        let Ok(name) = name() else {
            return true;
        };
        match ListenerOptions::new().name(name).create_sync() {
            Ok(listener) => {
                if let Ok(mut first) = FIRST.lock() {
                    *first = Some(listener);
                }
                return true;
            }
            Err(_) if hand_over(open).is_ok() => return false,
            // A socket file whose owner is gone: nobody answers, and the name stays taken until the file is removed.
            Err(_) => {
                let _ = std::fs::remove_file(socket_file());
            }
        }
    }
    true
}

pub fn watch(ctx: &egui::Context) -> Option<mpsc::Receiver<Option<PathBuf>>> {
    let listener = FIRST.lock().ok()?.take()?;
    let (arrived, arrivals) = mpsc::channel();
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut stream = BufReader::new(stream);
            let mut line = String::new();
            if stream.read_line(&mut line).is_err() {
                continue;
            }
            let _ = stream.get_mut().write_all(b"\n");
            let path = line.trim_end_matches(['\r', '\n']);
            let path = (!path.is_empty()).then(|| PathBuf::from(path));
            if arrived.send(path).is_err() {
                break;
            }
            ctx.request_repaint();
        }
    });
    Some(arrivals)
}
