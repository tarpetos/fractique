use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, mpsc};

use eframe::egui;
use fractique_core::{Control, Image, RenderError, RenderOptions, Scene, render};

pub const AS_SHOWN: u32 = 0;
pub const QUALITIES: [(&str, u32, u32); 4] = [
    ("q_instant", AS_SHOWN, 1),
    ("q_standard", 100, 1),
    ("q_high", 300, 2),
    ("q_ultra", 1000, 2),
];

#[derive(Default)]
pub struct Sheet {
    pub open: bool,
    pub quality: usize,
    pub size: Option<[u32; 2]>,
    pub custom: [String; 2],
    pub job: Option<Job>,
    pub note: String,
    pub share: bool,
}

pub enum Outcome {
    Saved(PathBuf),
    Cancelled,
    Failed(String),
}

pub struct Job {
    progress: Arc<AtomicU32>,
    cancel: Arc<AtomicBool>,
    done: mpsc::Receiver<Outcome>,
}

impl Job {
    pub fn start(
        scene: Scene,
        options: RenderOptions,
        path: PathBuf,
        ctx: egui::Context,
        pool: &'static rayon::ThreadPool,
    ) -> Self {
        let (progress, cancel) = (Arc::new(AtomicU32::new(0)), Arc::new(AtomicBool::new(false)));
        let (sender, done) = mpsc::channel();
        let (shown, stop) = (progress.clone(), cancel.clone());
        std::thread::spawn(move || {
            let report = |share: f64| {
                let permille = (share * 1000.0) as u32;
                if shown.swap(permille, Ordering::Relaxed) != permille {
                    ctx.request_repaint();
                }
            };
            let control = Control {
                cancel: Some(&stop),
                progress: Some(&report),
            };
            let outcome = match pool.install(|| render(&scene, &options, &control)) {
                Ok(image) => match crate::save_png(&path, &image.rgba, [image.width as usize, image.height as usize]) {
                    Ok(()) => Outcome::Saved(path),
                    Err(error) => Outcome::Failed(error),
                },
                Err(RenderError::Cancelled) => Outcome::Cancelled,
                Err(error) => Outcome::Failed(error.to_string()),
            };
            let _ = sender.send(outcome);
            ctx.request_repaint();
        });
        Self { progress, cancel, done }
    }

    pub fn save(image: Arc<Image>, path: PathBuf, ctx: egui::Context) -> Self {
        Self::made(move || image, path, ctx)
    }

    pub fn made(make: impl FnOnce() -> Arc<Image> + Send + 'static, path: PathBuf, ctx: egui::Context) -> Self {
        let (progress, cancel) = (Arc::new(AtomicU32::new(0)), Arc::new(AtomicBool::new(false)));
        let (sender, done) = mpsc::channel();
        let shown = progress.clone();
        std::thread::spawn(move || {
            let image = make();
            let outcome = match crate::save_png(&path, &image.rgba, [image.width as usize, image.height as usize]) {
                Ok(()) => Outcome::Saved(path),
                Err(error) => Outcome::Failed(error),
            };
            shown.store(1000, Ordering::Relaxed);
            let _ = sender.send(outcome);
            ctx.request_repaint();
        });
        Self { progress, cancel, done }
    }

    // An export that is saved when its picture comes from the graphics card.
    pub fn later(path: PathBuf, ctx: egui::Context) -> (Self, Later) {
        let (progress, cancel) = (Arc::new(AtomicU32::new(0)), Arc::new(AtomicBool::new(false)));
        let (sender, done) = mpsc::channel();
        let later = Later {
            path,
            progress: progress.clone(),
            cancel: cancel.clone(),
            sender,
            ctx,
        };
        (Self { progress, cancel, done }, later)
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn share(&self) -> f32 {
        self.progress.load(Ordering::Relaxed) as f32 / 1000.0
    }

    pub fn finished(&self) -> Option<Outcome> {
        self.done.try_recv().ok()
    }
}

pub struct Later {
    path: PathBuf,
    progress: Arc<AtomicU32>,
    cancel: Arc<AtomicBool>,
    sender: mpsc::Sender<Outcome>,
    ctx: egui::Context,
}

impl Later {
    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn save(self, mut rgba: Vec<u8>, size: [u32; 2]) {
        std::thread::spawn(move || {
            rgba.chunks_exact_mut(4).for_each(|pixel| pixel[3] = 255);
            let outcome = match crate::save_png(&self.path, &rgba, [size[0] as usize, size[1] as usize]) {
                Ok(()) => Outcome::Saved(self.path.clone()),
                Err(error) => Outcome::Failed(error),
            };
            self.progress.store(1000, Ordering::Relaxed);
            let _ = self.sender.send(outcome);
        });
    }

    pub fn fail(self) {
        let _ = self.sender.send(Outcome::Failed(String::new()));
    }
}

// Dropped without a picture, the export ends as a cancelled one: the sheet must not wait for it for ever.
impl Drop for Later {
    fn drop(&mut self) {
        let _ = self.sender.send(Outcome::Cancelled);
        self.ctx.request_repaint();
    }
}

pub fn target(name: &str) -> Option<PathBuf> {
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        rfd::FileDialog::new()
            .set_file_name(name)
            .add_filter("PNG", &["png"])
            .save_file()
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        let folder = crate::own_folder()?.join("exports");
        std::fs::create_dir_all(&folder).ok()?;
        Some(folder.join(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_job_writes_the_picture() {
        static POOL: std::sync::OnceLock<rayon::ThreadPool> = std::sync::OnceLock::new();
        let pool = POOL.get_or_init(|| rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap());
        let path = std::env::temp_dir().join(format!("fractique-export-test-{}.png", std::process::id()));
        let options = RenderOptions {
            width: 64,
            height: 36,
            samples_per_pixel: 8,
            supersample: 1,
        };
        let scene = fractique_core::presets::get("aurora-spiral").unwrap();
        let job = Job::start(scene, options, path.clone(), egui::Context::default(), pool);
        let outcome = job.done.recv().unwrap();
        assert!(matches!(outcome, Outcome::Saved(_)));
        assert_eq!(job.share(), 1.0);
        let written = std::fs::read(&path).unwrap();
        assert!(written.starts_with(&[0x89, b'P', b'N', b'G']));
        let _ = std::fs::remove_file(path);
    }
}
