use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, mpsc};

use eframe::egui;
use fractique_core::render::{MAX_CELLS, MAX_SIDE};
use fractique_core::{Control, Image, RenderError, RenderOptions, Scene, render};
use web_time::Instant;

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

// Why an export did not end in a file: each has a text of its own, and the error itself goes to the log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    TooLarge,
    Scene,
    File,
    Picture,
}

impl Failure {
    pub fn key(self) -> &'static str {
        match self {
            Failure::TooLarge => "export_too_large",
            Failure::Scene => "export_scene",
            Failure::File => "export_file",
            Failure::Picture => "export_picture",
        }
    }

    fn of(error: &RenderError) -> Self {
        log::warn!("the export was not rendered: {error}");
        match error {
            RenderError::Options(_) => Failure::TooLarge,
            _ => Failure::Scene,
        }
    }
}

pub enum Outcome {
    Saved(PathBuf),
    Cancelled,
    Failed(Failure),
}

// Whether the renderer takes a picture of this size at this supersample: its own limits, asked before the press.
pub fn fits([width, height]: [u32; 2], supersample: u32) -> bool {
    let cells = u64::from(width) * u64::from(height) * u64::from(supersample) * u64::from(supersample);
    width.max(height) <= MAX_SIDE && cells <= MAX_CELLS
}

// How an Instant export of a size comes from the picture on the screen: saved as it is, made smaller, or drawn
// again, in tiles on the graphics card where it is larger and the card draws the scene pixel by pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    Shown,
    Shrunk,
    Larger,
    Other,
}

// Two sizes have one shape when a side of one, scaled to the other, is the other's side within a pixel.
pub fn same_shape(one: [u32; 2], other: [u32; 2]) -> bool {
    let scaled = f64::from(one[1]) * f64::from(other[0]) / f64::from(one[0].max(1));
    (scaled - f64::from(other[1])).abs() <= 1.0
}

pub fn way(screen: [u32; 2], size: [u32; 2]) -> Way {
    if screen == size {
        Way::Shown
    } else if same_shape(screen, size) && size[0] < screen[0] {
        Way::Shrunk
    } else if size[0] >= screen[0] && size[1] >= screen[1] {
        Way::Larger
    } else {
        Way::Other
    }
}

// The text that says, before the press, why an Instant export of this size is not instant: nothing when it is.
pub fn slow(screen: [u32; 2], size: [u32; 2]) -> Option<&'static str> {
    match way(screen, size) {
        Way::Shown | Way::Shrunk => None,
        _ if same_shape(screen, size) => Some("export_larger"),
        _ => Some("export_shape"),
    }
}

// A tile of an export drawn on the graphics card is at most this long a side, and is drawn with so many pixels
// more around it, which are cut off: a pixel at its edge has the neighbours it has in the whole picture.
pub const TILE: u32 = 2048;
pub const APRON: u32 = 8;

// The tiles of a picture, row after row, as [left, top, width, height], and how many stand in a row.
pub fn tiles([width, height]: [u32; 2], most: u32) -> (usize, Vec<[u32; 4]>) {
    let cut = |side: u32| -> Vec<[u32; 2]> {
        let count = side.div_ceil(most.max(1)).max(1);
        let each = side.div_ceil(count);
        (0..count)
            .map(|at| [at * each, each.min(side - at * each)])
            .filter(|[_, long]| *long > 0)
            .collect()
    };
    let (columns, rows) = (cut(width), cut(height));
    let cells = rows
        .iter()
        .flat_map(|[top, tall]| columns.iter().map(move |[left, wide]| [*left, *top, *wide, *tall]))
        .collect();
    (columns.len(), cells)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Rendering,
    Writing,
}

// The seconds a pixel of the file takes to write on its one thread: 10.7 ns for a flame of 3840 × 2160 on a
// Ryzen 7 5700X. It is what share of the bar the writing gets beside a render that took so long.
const WRITING: f64 = 1.2e-8;
// Rows are handed to the encoder so many bytes at a time: each is a step of the bar and a look at Cancel.
const STRIP: usize = 4 << 20;

// The seconds a cell of the picture takes to be developed once its samples are in (the density estimation and the
// tone, of which the renderer tells no progress): 4.0 s for a flame of 3840 × 2160 on the ten threads of the pool.
const DEVELOPING: f64 = 5.0e-7;

// How far an export is, the render and the writing of the file in one: a thousandth at a step.
struct Bar {
    permille: AtomicU32,
    writing: AtomicBool,
    cancel: AtomicBool,
    began: Instant,
    // The milliseconds after which the samples were in; nothing before that.
    sampled: AtomicU32,
    // The seconds the developing and the writing are expected to take.
    developing: f64,
    filing: f64,
}

// The share of the whole job that is done when the render is `share` done after `spent` seconds and the file is
// expected to take `writing` seconds.
fn rendered_share(share: f64, spent: f64, writing: f64) -> f64 {
    if share <= 0.0 {
        return 0.0;
    }
    spent / (spent / share.min(1.0) + writing).max(1e-9)
}

// The same once the samples took `sampled` seconds and the developing has gone on for `spent` of the seconds it
// is expected to take.
fn developed_share(sampled: f64, spent: f64, developing: f64, filing: f64) -> f64 {
    (sampled + spent.clamp(0.0, developing)) / (sampled + developing + filing).max(1e-9)
}

// The same once the render has taken `rendered` seconds and `rows` of the file are written.
fn written_share(rows: f64, rendered: f64, writing: f64) -> f64 {
    (rendered + writing * rows) / (rendered + writing).max(1e-9)
}

impl Bar {
    fn new([width, height]: [u32; 2], supersample: u32, rendered: bool) -> Arc<Self> {
        let pixels = f64::from(width) * f64::from(height);
        let cells = pixels * f64::from(supersample * supersample);
        Arc::new(Self {
            permille: AtomicU32::new(0),
            writing: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            began: Instant::now(),
            sampled: AtomicU32::new(0),
            developing: if rendered { cells * DEVELOPING } else { 0.0 },
            filing: pixels * WRITING,
        })
    }

    // What the renderer tells: the share of the samples that are in.
    fn sample(&self, share: f64, ctx: &egui::Context) {
        let spent = self.began.elapsed().as_secs_f64();
        self.show(rendered_share(share, spent, self.developing + self.filing), ctx);
        // The last the renderer tells of is a batch of samples short of all of them.
        if share >= 0.99 {
            let _ = self.sampled.compare_exchange(
                0,
                ((spent * 1000.0) as u32).max(1),
                Ordering::Relaxed,
                Ordering::Relaxed,
            );
        }
    }

    // How far the whole job is now. Between the last sample and the file the renderer is silent: the bar goes by
    // the time the developing is expected to take, and waits at its end if it takes longer.
    fn share(&self) -> f32 {
        let sampled = f64::from(self.sampled.load(Ordering::Relaxed)) / 1000.0;
        if sampled > 0.0 && !self.writing.load(Ordering::Relaxed) {
            let spent = self.began.elapsed().as_secs_f64();
            let shown = developed_share(sampled, spent - sampled, self.developing, self.filing);
            self.permille
                .fetch_max(((shown * 1000.0) as u32).min(999), Ordering::Relaxed);
        }
        self.permille.load(Ordering::Relaxed) as f32 / 1000.0
    }

    fn developing(&self) -> bool {
        self.sampled.load(Ordering::Relaxed) > 0 && !self.writing.load(Ordering::Relaxed)
    }

    // The bar never goes back, and is full only when the file is on the disk.
    fn show(&self, share: f64, ctx: &egui::Context) {
        let permille = ((share * 1000.0) as u32).min(999);
        if self.permille.fetch_max(permille, Ordering::Relaxed) < permille {
            ctx.request_repaint();
        }
    }

    // Writes the file, a strip of rows at a time, after a render that took `rendered` seconds.
    fn write(&self, path: &Path, rgba: &[u8], size: [u32; 2], ctx: &egui::Context) -> Outcome {
        let rendered = if self.developing > 0.0 {
            self.began.elapsed().as_secs_f64()
        } else {
            0.0
        };
        self.writing.store(true, Ordering::Relaxed);
        ctx.request_repaint();
        let rows = |share: f64| self.show(written_share(share, rendered, self.filing), ctx);
        match write_png(path, rgba, size, &self.cancel, &rows) {
            Ok(true) => {
                self.permille.store(1000, Ordering::Relaxed);
                Outcome::Saved(path.to_owned())
            }
            Ok(false) => Outcome::Cancelled,
            Err(error) => {
                log::warn!("the export was not written: {error}");
                Outcome::Failed(Failure::File)
            }
        }
    }
}

// The picture as a PNG, compressed fast: false when it was cancelled. A file that is not whole is removed.
pub(crate) fn write_png(
    path: &Path,
    rgba: &[u8],
    size: [u32; 2],
    cancel: &AtomicBool,
    rows: &dyn Fn(f64),
) -> Result<bool, String> {
    encode(path, rgba, size, png::Compression::Fast, cancel, rows)
}

fn encode(
    path: &Path,
    rgba: &[u8],
    [width, height]: [u32; 2],
    compression: png::Compression,
    cancel: &AtomicBool,
    rows: &dyn Fn(f64),
) -> Result<bool, String> {
    let written = (|| {
        let file = std::fs::File::create(path).map_err(|error| error.to_string())?;
        let mut encoder = png::Encoder::new(std::io::BufWriter::with_capacity(1 << 20, file), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(compression);
        let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
        let mut stream = writer.stream_writer().map_err(|error| error.to_string())?;
        let row = width as usize * 4;
        let strip = (STRIP / row.max(1)).max(1) * row.max(1);
        let all = rgba.len().max(1) as f64;
        for (index, rows_of) in rgba.chunks(strip).enumerate() {
            if cancel.load(Ordering::Relaxed) {
                return Ok(false);
            }
            stream.write_all(rows_of).map_err(|error| error.to_string())?;
            rows(((index * strip + rows_of.len()) as f64 / all).min(1.0));
        }
        stream.finish().map_err(|error| error.to_string())?;
        writer.finish().map_err(|error| error.to_string())?;
        Ok(true)
    })();
    if !matches!(written, Ok(true)) {
        let _ = std::fs::remove_file(path);
    }
    written
}

pub struct Job {
    bar: Arc<Bar>,
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
        let bar = Bar::new([options.width, options.height], options.supersample, true);
        let (sender, done) = mpsc::channel();
        let shown = bar.clone();
        std::thread::spawn(move || {
            let report = |share: f64| shown.sample(share, &ctx);
            let control = Control {
                cancel: Some(&shown.cancel),
                progress: Some(&report),
            };
            let outcome = match pool.install(|| render(&scene, &options, &control)) {
                Ok(image) => shown.write(&path, &image.rgba, [image.width, image.height], &ctx),
                Err(RenderError::Cancelled) => Outcome::Cancelled,
                Err(error) => Outcome::Failed(Failure::of(&error)),
            };
            let _ = sender.send(outcome);
            ctx.request_repaint();
        });
        Self { bar, done }
    }

    pub fn save(image: Arc<Image>, path: PathBuf, ctx: egui::Context) -> Self {
        Self::made([image.width, image.height], move || image, path, ctx)
    }

    pub fn made(
        size: [u32; 2],
        make: impl FnOnce() -> Arc<Image> + Send + 'static,
        path: PathBuf,
        ctx: egui::Context,
    ) -> Self {
        let bar = Bar::new(size, 1, false);
        let (sender, done) = mpsc::channel();
        let shown = bar.clone();
        std::thread::spawn(move || {
            let image = make();
            let outcome = shown.write(&path, &image.rgba, [image.width, image.height], &ctx);
            let _ = sender.send(outcome);
            ctx.request_repaint();
        });
        Self { bar, done }
    }

    // An export that is saved when its picture comes from the graphics card.
    pub fn later(size: [u32; 2], path: PathBuf, ctx: egui::Context) -> (Self, Later) {
        let bar = Bar::new(size, 1, false);
        let (sender, done) = mpsc::channel();
        let later = Later {
            path,
            bar: bar.clone(),
            sender,
            ctx,
        };
        (Self { bar, done }, later)
    }

    // An export whose picture comes in tiles, row after row: each row of tiles is written when it is whole, so
    // the picture is never in one piece anywhere but in the file.
    pub fn tiled(
        size: [u32; 2],
        across: usize,
        cells: Vec<[u32; 4]>,
        path: PathBuf,
        ctx: egui::Context,
    ) -> (Self, Tiles) {
        let bar = Bar::new(size, 1, false);
        let (sender, done) = mpsc::channel();
        let (tiles, landed) = mpsc::channel::<(usize, Vec<u8>)>();
        let (shown, at) = (bar.clone(), path.clone());
        std::thread::spawn(move || {
            let outcome = match stitched(&at, size, across, &cells, &landed, &shown, &ctx) {
                Ok(true) => {
                    shown.permille.store(1000, Ordering::Relaxed);
                    Outcome::Saved(at)
                }
                Ok(false) => Outcome::Cancelled,
                Err(error) => {
                    log::warn!("the export was not written: {error}");
                    Outcome::Failed(Failure::File)
                }
            };
            let _ = sender.send(outcome);
            ctx.request_repaint();
        });
        (
            Self { bar: bar.clone(), done },
            Tiles {
                path,
                bar,
                sender: tiles,
            },
        )
    }

    pub fn cancel(&self) {
        self.bar.cancel.store(true, Ordering::Relaxed);
    }

    pub fn share(&self) -> f32 {
        self.bar.share()
    }

    // The bar moves by the clock, not by what the renderer tells: the sheet draws it again without being asked.
    pub fn timed(&self) -> bool {
        self.bar.developing()
    }

    pub fn stage(&self) -> Stage {
        if self.bar.writing.load(Ordering::Relaxed) {
            Stage::Writing
        } else {
            Stage::Rendering
        }
    }

    pub fn finished(&self) -> Option<Outcome> {
        self.done.try_recv().ok()
    }
}

// Where the tiles of an export go as they come back from the graphics card, in any order.
pub struct Tiles {
    path: PathBuf,
    bar: Arc<Bar>,
    sender: mpsc::Sender<(usize, Vec<u8>)>,
}

impl Tiles {
    pub fn cancelled(&self) -> bool {
        self.bar.cancel.load(Ordering::Relaxed)
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn sender(&self) -> mpsc::Sender<(usize, Vec<u8>)> {
        self.sender.clone()
    }
}

// Writes the file from tiles that arrive in any order: false when it was cancelled or the tiles stopped coming.
// The file is begun with the first row of tiles, and one that is not whole is removed.
fn stitched(
    path: &Path,
    [width, height]: [u32; 2],
    across: usize,
    cells: &[[u32; 4]],
    landed: &mpsc::Receiver<(usize, Vec<u8>)>,
    bar: &Bar,
    ctx: &egui::Context,
) -> Result<bool, String> {
    let mut begun = false;
    let written = (|| {
        let mut waiting: std::collections::HashMap<usize, Vec<u8>> = std::collections::HashMap::new();
        let mut stream = None;
        let mut rows_done = 0u32;
        for (row, row_cells) in cells.chunks(across.max(1)).enumerate() {
            let tall = row_cells[0][3] as usize;
            let mut strip = vec![255u8; width as usize * tall * 4];
            for (column, [left, _, wide, _]) in row_cells.iter().enumerate() {
                let index = row * across + column;
                let tile = loop {
                    if let Some(tile) = waiting.remove(&index) {
                        break tile;
                    }
                    match landed.recv() {
                        Ok((index, tile)) => waiting.insert(index, tile),
                        Err(_) => return Ok(false),
                    };
                    if bar.cancel.load(Ordering::Relaxed) {
                        return Ok(false);
                    }
                };
                let (left, wide) = (*left as usize * 4, *wide as usize * 4);
                if tile.len() != wide * tall {
                    return Err("a tile of another size".to_owned());
                }
                for (y, pixels) in tile.chunks_exact(wide).enumerate() {
                    let at = y * width as usize * 4 + left;
                    strip[at..at + wide].copy_from_slice(pixels);
                }
            }
            strip.chunks_exact_mut(4).for_each(|pixel| pixel[3] = 255);
            if stream.is_none() {
                let file = std::fs::File::create(path).map_err(|error| error.to_string())?;
                begun = true;
                let mut encoder = png::Encoder::new(std::io::BufWriter::with_capacity(1 << 20, file), width, height);
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder.set_compression(png::Compression::Fast);
                let writer = encoder.write_header().map_err(|error| error.to_string())?;
                stream = Some(writer.into_stream_writer().map_err(|error| error.to_string())?);
                bar.writing.store(true, Ordering::Relaxed);
            }
            let Some(stream) = &mut stream else {
                return Ok(false);
            };
            let each = (STRIP / (width as usize * 4).max(1)).max(1) * width as usize * 4;
            for rows in strip.chunks(each) {
                if bar.cancel.load(Ordering::Relaxed) {
                    return Ok(false);
                }
                stream.write_all(rows).map_err(|error| error.to_string())?;
                rows_done += (rows.len() / (width as usize * 4)) as u32;
                bar.show(f64::from(rows_done) / f64::from(height.max(1)), ctx);
            }
        }
        match stream {
            Some(stream) => stream.finish().map_err(|error| error.to_string())?,
            None => return Ok(false),
        }
        Ok(true)
    })();
    if begun && !matches!(written, Ok(true)) {
        let _ = std::fs::remove_file(path);
    }
    written
}

pub struct Later {
    path: PathBuf,
    bar: Arc<Bar>,
    sender: mpsc::Sender<Outcome>,
    ctx: egui::Context,
}

impl Later {
    pub fn cancelled(&self) -> bool {
        self.bar.cancel.load(Ordering::Relaxed)
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn save(self, mut rgba: Vec<u8>, size: [u32; 2]) {
        std::thread::spawn(move || {
            rgba.chunks_exact_mut(4).for_each(|pixel| pixel[3] = 255);
            let outcome = self.bar.write(&self.path, &rgba, size, &self.ctx);
            let _ = self.sender.send(outcome);
            self.ctx.request_repaint();
        });
    }

    // The picture of the screen made smaller, to the size of the export, and saved.
    pub fn shrunk(self, rgba: Vec<u8>, from: [u32; 2], to: [u32; 2]) {
        std::thread::spawn(move || {
            let mut small = crate::pace::shrunk(&rgba, from, to);
            drop(rgba);
            small.chunks_exact_mut(4).for_each(|pixel| pixel[3] = 255);
            let outcome = self.bar.write(&self.path, &small, to, &self.ctx);
            let _ = self.sender.send(outcome);
            self.ctx.request_repaint();
        });
    }

    pub fn fail(self) {
        let _ = self.sender.send(Outcome::Failed(Failure::Picture));
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

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("fractique-{name}-{}.png", std::process::id()))
    }

    fn decoded(path: &Path) -> (Vec<u8>, [u32; 2]) {
        let mut decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
        decoder.set_transformations(png::Transformations::IDENTITY);
        let mut reader = decoder.read_info().unwrap();
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut pixels).unwrap();
        pixels.truncate(info.buffer_size());
        (pixels, [info.width, info.height])
    }

    fn picture(width: u32, height: u32) -> Vec<u8> {
        let mut state = 0x9e37_79b9_u32;
        (0..width * height * 4)
            .map(|index| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                if index % 4 == 3 {
                    255
                } else {
                    (state >> 24) as u8 & 0xf8
                }
            })
            .collect()
    }

    #[test]
    fn an_instant_export_is_the_screen_s_picture_where_it_has_its_shape_and_is_no_larger() {
        let screen = [3840, 2160];
        assert_eq!(way(screen, [3840, 2160]), Way::Shown);
        for size in [[1920, 1080], [1280, 720], [2560, 1440], [3839, 2159], [1366, 768]] {
            assert_eq!(way(screen, size), Way::Shrunk, "{size:?}");
            assert_eq!(slow(screen, size), None);
        }
        for size in [[7680, 4320], [15360, 8640], [3841, 2161]] {
            assert_eq!(way(screen, size), Way::Larger, "{size:?}");
            assert_eq!(slow(screen, size), Some("export_larger"));
        }
        for size in [
            [1080, 1920],
            [3440, 1440],
            [2160, 2160],
            [1920, 1200],
            [16384, 16384],
            [3840, 2400],
        ] {
            assert!(matches!(way(screen, size), Way::Larger | Way::Other), "{size:?}");
            assert_eq!(slow(screen, size), Some("export_shape"), "{size:?}");
        }
        assert_eq!(slow(screen, screen), None);
        assert!(same_shape([3840, 2160], [1366, 768]) && !same_shape([3840, 2160], [1366, 770]));
    }

    #[test]
    fn the_tiles_of_a_picture_cover_it_once() {
        for size in [
            [15360, 8640],
            [7680, 4320],
            [5000, 3001],
            [2048, 2048],
            [2049, 100],
            [16384, 16384],
        ] {
            let (across, cells) = tiles(size, TILE);
            assert_eq!(cells.len() % across, 0);
            let mut covered = 0u64;
            for (index, [left, top, wide, tall]) in cells.iter().enumerate() {
                assert!(*wide <= TILE && *tall <= TILE && *wide > 0 && *tall > 0);
                assert!(left + wide <= size[0] && top + tall <= size[1]);
                covered += u64::from(*wide) * u64::from(*tall);
                if index % across > 0 {
                    let before = cells[index - 1];
                    assert_eq!((before[0] + before[2], before[1], before[3]), (*left, *top, *tall));
                }
            }
            assert_eq!(covered, u64::from(size[0]) * u64::from(size[1]), "{size:?}");
        }
        assert_eq!(tiles([15360, 8640], TILE).1.len(), 40);
    }

    #[test]
    fn a_file_stitched_from_tiles_holds_the_very_pixels() {
        let size = [700u32, 500];
        let rgba = picture(size[0], size[1]);
        let (across, cells) = tiles(size, 256);
        let path = temp("tiles");
        let (job, to) = Job::tiled(size, across, cells.clone(), path.clone(), egui::Context::default());
        // In another order than the file's.
        for (index, [left, top, wide, tall]) in cells.iter().enumerate().rev() {
            let mut tile = Vec::new();
            for y in *top..top + tall {
                let at = ((y * size[0] + left) * 4) as usize;
                tile.extend_from_slice(&rgba[at..at + *wide as usize * 4]);
            }
            to.sender().send((index, tile)).unwrap();
        }
        let outcome = job.done.recv().unwrap();
        assert!(matches!(outcome, Outcome::Saved(_)));
        let (pixels, read) = decoded(&path);
        assert_eq!(read, size);
        assert!(pixels == rgba);
        let _ = std::fs::remove_file(&path);
        // Tiles that stop coming leave no file.
        let (job, to) = Job::tiled(size, across, cells, path.clone(), egui::Context::default());
        to.sender().send((0, vec![0; 234 * 250 * 4])).unwrap();
        drop(to);
        assert!(matches!(job.done.recv().unwrap(), Outcome::Cancelled));
        assert!(!path.exists());
    }

    #[test]
    fn a_file_written_in_strips_holds_the_very_pixels() {
        // Wider than a strip is long, so the file is written in several.
        let (width, height) = (1500, 1800);
        let rgba = picture(width, height);
        let path = temp("strips");
        let steps = std::sync::Mutex::new(Vec::new());
        let rows = |share: f64| steps.lock().unwrap().push(share);
        let written = write_png(&path, &rgba, [width, height], &AtomicBool::new(false), &rows);
        assert_eq!(written, Ok(true));
        let steps = steps.into_inner().unwrap();
        assert!(steps.len() > 2 && steps.windows(2).all(|pair| pair[1] > pair[0]));
        assert_eq!(steps.last(), Some(&1.0));
        let (pixels, size) = decoded(&path);
        assert_eq!(size, [width, height]);
        assert!(pixels == rgba);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn an_export_cancelled_while_its_file_is_written_leaves_no_file() {
        let (width, height) = (1500, 1800);
        let rgba = picture(width, height);
        let path = temp("cancelled");
        let cancel = AtomicBool::new(false);
        let rows = |share: f64| cancel.store(share > 0.3, Ordering::Relaxed);
        assert_eq!(write_png(&path, &rgba, [width, height], &cancel, &rows), Ok(false));
        assert!(!path.exists());
        let nowhere = std::env::temp_dir().join("fractique-no-such-folder").join("x.png");
        assert!(write_png(&nowhere, &rgba, [width, height], &cancel, &rows).is_err());
    }

    #[test]
    fn the_bar_covers_the_render_and_the_file_and_never_goes_back() {
        // A render of 10 s and a file of 5 s: the render ends at two thirds of the bar, where the file begins.
        let mut last = 0.0;
        for step in 1..=100 {
            let share = f64::from(step) / 100.0;
            let shown = rendered_share(share, 10.0 * share, 5.0);
            assert!(shown >= last && shown <= 2.0 / 3.0 + 1e-9, "{shown}");
            last = shown;
        }
        assert!((last - 2.0 / 3.0).abs() < 1e-9);
        assert!((written_share(0.0, 10.0, 5.0) - last).abs() < 1e-9);
        assert!((written_share(0.5, 10.0, 5.0) - 5.0 / 6.0).abs() < 1e-9);
        assert_eq!(written_share(1.0, 10.0, 5.0), 1.0);
        // A picture that is only written is all file.
        assert_eq!(written_share(0.25, 0.0, 5.0), 0.25);
        assert_eq!(rendered_share(0.0, 0.0, 5.0), 0.0);
        // Shown, it is full only once the file is whole.
        let (bar, ctx) = (Bar::new([64, 36], 1, true), egui::Context::default());
        bar.show(1.0, &ctx);
        bar.show(0.4, &ctx);
        assert_eq!(bar.permille.load(Ordering::Relaxed), 999);
        // Samples of 6 s, a developing of 3 s and a file of 1 s: the bar goes on by the clock while the renderer
        // is silent, stops where the developing should end, and the file takes it from there.
        assert!((rendered_share(1.0, 6.0, 4.0) - 0.6).abs() < 1e-9);
        assert!((developed_share(6.0, 0.0, 3.0, 1.0) - 0.6).abs() < 1e-9);
        assert!((developed_share(6.0, 1.5, 3.0, 1.0) - 0.75).abs() < 1e-9);
        assert!((developed_share(6.0, 30.0, 3.0, 1.0) - 0.9).abs() < 1e-9);
        assert!((written_share(0.0, 9.0, 1.0) - 0.9).abs() < 1e-9);
        let job = Bar::new([64, 36], 1, true);
        assert!(!job.developing());
        job.sample(1.0, &ctx);
        assert!(job.developing() && job.share() < 1.0);
    }

    #[test]
    fn a_quality_is_offered_where_the_renderer_takes_the_size() {
        let fitting = |size| QUALITIES.map(|(_, _, supersample)| fits(size, supersample));
        assert_eq!(fitting([3840, 2160]), [true; 4]);
        assert_eq!(fitting([7680, 4320]), [true; 4]);
        assert_eq!(fitting([15360, 8640]), [true, true, false, false]);
        assert_eq!(fitting([16384, 16384]), [true, true, false, false]);
        assert!(!fits([16385, 10], 1));
        // The renderer itself says the same.
        for (size, supersample) in [
            ([64, 36], 2),
            ([15360, 8640], 2),
            ([8193, 8192], 2),
            ([16384, 16384], 4),
        ] {
            let options = RenderOptions {
                width: size[0],
                height: size[1],
                samples_per_pixel: 1,
                supersample,
            };
            let scene = fractique_core::presets::get("aurora-spiral").unwrap();
            let stop = AtomicBool::new(true);
            let control = Control {
                cancel: Some(&stop),
                progress: None,
            };
            let refused = matches!(render(&scene, &options, &control), Err(RenderError::Options(_)));
            assert_eq!(!refused, fits(size, supersample), "{size:?} {supersample}");
        }
    }

    #[test]
    fn every_failure_has_a_text_in_every_language() {
        for failure in [Failure::TooLarge, Failure::Scene, Failure::File, Failure::Picture] {
            for lang in crate::i18n::Lang::ALL {
                let text = crate::i18n::Texts::new(lang).get(failure.key());
                assert!(
                    !text.is_empty() && text != failure.key(),
                    "{} {}",
                    lang.code(),
                    failure.key()
                );
            }
        }
    }

    // FRACTIQUE_BENCH=3840x2160:100 cargo test --release -p fractique-app export::tests::the_time -- --ignored --nocapture
    #[test]
    #[ignore = "prints the seconds of a render and of each way to write its file"]
    fn the_time_of_a_render_and_of_its_file() {
        let asked = std::env::var("FRACTIQUE_BENCH").unwrap_or_else(|_| "3840x2160:100".to_owned());
        let numbers: Vec<u32> = asked
            .split(['x', ':'])
            .filter_map(|number| number.parse().ok())
            .collect();
        let [width, height, samples] = numbers[..] else {
            panic!("FRACTIQUE_BENCH=3840x2160:100");
        };
        let name = std::env::var("FRACTIQUE_BENCH_SCENE").unwrap_or_else(|_| "aurora-spiral".to_owned());
        let scene = fractique_core::presets::get(&name)
            .or_else(|| fractique_core::fractal::preset(&name))
            .expect("a preset");
        let options = RenderOptions {
            width,
            height,
            samples_per_pixel: samples,
            supersample: 1,
        };
        let began = Instant::now();
        let image = render(&scene, &options, &Control::default()).unwrap();
        println!(
            "{name} {width} x {height}, {samples} samples: rendered in {:.1} s",
            began.elapsed().as_secs_f64()
        );
        let path = temp("bench");
        let stop = AtomicBool::new(false);
        let mut first: Option<Vec<u8>> = None;
        for compression in [
            png::Compression::Balanced,
            png::Compression::Fast,
            png::Compression::Fastest,
        ] {
            let began = Instant::now();
            encode(&path, &image.rgba, [width, height], compression, &stop, &|_| {}).unwrap();
            let seconds = began.elapsed().as_secs_f64();
            let bytes = std::fs::metadata(&path).unwrap().len();
            let (pixels, _) = decoded(&path);
            assert!(pixels == image.rgba);
            println!(
                "{compression:?}: {seconds:.1} s, {:.1} MB, {:.1} ns a pixel, the pixels are the same",
                bytes as f64 / 1e6,
                seconds * 1e9 / (f64::from(width) * f64::from(height))
            );
            first.get_or_insert(pixels);
        }
        let _ = std::fs::remove_file(path);
    }

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
