use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, mpsc};

use eframe::egui;
use fractique_core::{Control, RenderError, RenderOptions, Scene, render};

pub const SIZES: [[u32; 2]; 5] = [[1920, 1080], [2560, 1440], [3840, 2160], [1080, 1920], [1080, 2400]];
pub const QUALITIES: [(&str, u32, u32); 3] = [("q_standard", 100, 1), ("q_high", 300, 2), ("q_ultra", 1000, 2)];

#[derive(Default)]
pub struct Sheet {
    pub open: bool,
    pub size: usize,
    pub quality: usize,
    pub job: Option<Job>,
    pub note: String,
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
