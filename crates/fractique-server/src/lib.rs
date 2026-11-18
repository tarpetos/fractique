mod mcp;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::header::{CONTENT_TYPE, RETRY_AFTER};
use axum::http::{HeaderName, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use fractique_core::random::random_scene;
use fractique_core::wallpaper::{Layout, Material, Request, random_wallpaper};
use fractique_core::{Control, Quality, RenderError, RenderOptions, Scene, fractal, presets, render};
pub use fractique_mcp::encode_png;
use fractique_mcp::{Host, Limits};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, watch};
use tower_http::services::ServeDir;

pub const MAX_BODY_BYTES: usize = 1 << 20;
const HASH_HEADER: HeaderName = HeaderName::from_static("x-fractique-hash");

#[derive(Clone, Debug)]
pub struct Config {
    pub jobs: usize,
    pub queue: usize,
    pub max_side: u32,
    pub max_pixels: u64,
    pub max_samples: u32,
    pub max_supersample: u32,
    pub timeout: Duration,
    pub web: Option<PathBuf>,
    pub mcp: bool,
    pub mcp_origins: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            jobs: 1,
            queue: 8,
            max_side: 4096,
            max_pixels: 3840 * 2160,
            max_samples: 1000,
            max_supersample: 2,
            timeout: Duration::from_secs(120),
            web: None,
            mcp: true,
            mcp_origins: Vec::new(),
        }
    }
}

impl Config {
    fn limits(&self) -> Limits {
        Limits {
            max_side: self.max_side,
            max_pixels: self.max_pixels,
            max_samples: self.max_samples,
            max_supersample: self.max_supersample,
        }
    }
}

struct Shared {
    config: Config,
    slots: Arc<Semaphore>,
    busy: watch::Sender<usize>,
    host: Host,
    sessions: Mutex<mcp::Sessions>,
}

struct Ticket(Arc<Shared>);

impl Ticket {
    fn take(shared: &Arc<Shared>) -> Option<Self> {
        let room = shared.config.jobs + shared.config.queue;
        shared
            .busy
            .send_if_modified(|busy| {
                let admitted = *busy < room;
                *busy += usize::from(admitted);
                admitted
            })
            .then(|| Self(shared.clone()))
    }
}

impl Drop for Ticket {
    fn drop(&mut self) {
        self.0.busy.send_modify(|busy| *busy -= 1);
    }
}

struct CancelOnDrop(Arc<AtomicBool>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

struct Failure(StatusCode, String);

impl Failure {
    fn bad(message: impl Into<String>) -> Self {
        Self(StatusCode::BAD_REQUEST, message.into())
    }

    fn internal(message: impl ToString) -> Self {
        Self(StatusCode::INTERNAL_SERVER_ERROR, message.to_string())
    }
}

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        let body = Json(json!({ "error": self.1 }));
        if self.0 == StatusCode::TOO_MANY_REQUESTS {
            (self.0, [(RETRY_AFTER, "1")], body).into_response()
        } else {
            (self.0, body).into_response()
        }
    }
}

fn parse<T: DeserializeOwned>(body: &Bytes) -> Result<T, Failure> {
    serde_json::from_slice(body).map_err(|e| Failure::bad(format!("not a valid request: {e}")))
}

fn preset(name: &str) -> Result<Scene, Failure> {
    fractique_mcp::preset(name).map_err(|message| Failure(StatusCode::NOT_FOUND, message))
}

fn kind(name: Option<String>) -> Result<Option<String>, Failure> {
    match name {
        Some(name) if fractal::spec(&name).is_none() => Err(Failure::bad(format!("no fractal is called '{name}'"))),
        name => Ok(name),
    }
}

fn random_of(kind: Option<&str>, seed: u64) -> Scene {
    kind.and_then(|kind| fractal::random_scene(kind, seed))
        .unwrap_or_else(|| random_scene(seed))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RandomBody {
    seed: u64,
    fractal: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WallpaperBody {
    seed: u64,
    layout: Option<String>,
    material: Option<String>,
    #[serde(default)]
    dark: bool,
    hue: Option<f64>,
    width: Option<u32>,
    height: Option<u32>,
}

impl WallpaperBody {
    fn request(&self, width: u32, height: u32) -> Result<Request, Failure> {
        let (width, height) = (self.width.unwrap_or(width), self.height.unwrap_or(height));
        if width == 0 || height == 0 {
            return Err(Failure::bad("width and height must be at least 1"));
        }
        let layout = self
            .layout
            .as_deref()
            .map(|name| Layout::parse(name).ok_or_else(|| Failure::bad(format!("no layout is called '{name}'"))));
        let material = self
            .material
            .as_deref()
            .map(|name| Material::parse(name).ok_or_else(|| Failure::bad(format!("no material is called '{name}'"))));
        Ok(Request {
            layout: layout.transpose()?,
            material: material.transpose()?,
            ratio: f64::from(height) / f64::from(width),
            dark: self.dark,
            hue: self.hue,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenderBody {
    scene: Option<Value>,
    preset: Option<String>,
    random: Option<u64>,
    fractal: Option<String>,
    wallpaper: Option<WallpaperBody>,
    width: u32,
    height: u32,
    quality: Option<String>,
    samples: Option<u32>,
    supersample: Option<u32>,
    seed: Option<u64>,
}

enum Source {
    Scene(Box<Scene>),
    Random(u64, Option<String>),
    Wallpaper(u64, Request),
}

impl RenderBody {
    fn options(&self, limits: &Config) -> Result<RenderOptions, Failure> {
        let quality = self.quality.as_deref().unwrap_or("standard");
        let quality =
            Quality::parse(quality).ok_or_else(|| Failure::bad(format!("no quality is called '{quality}'")))?;
        let mut options = RenderOptions::new(self.width, self.height, quality);
        options.samples_per_pixel = self.samples.unwrap_or(options.samples_per_pixel);
        options.supersample = self.supersample.unwrap_or(options.supersample);

        limits.limits().check(&options).map_err(Failure::bad)?;
        Ok(options)
    }

    fn source(self) -> Result<Source, Failure> {
        let given = [
            self.scene.is_some(),
            self.preset.is_some(),
            self.random.is_some(),
            self.wallpaper.is_some(),
        ];
        if given.into_iter().filter(|given| *given).count() != 1 {
            return Err(Failure::bad("give exactly one of scene, preset, random or wallpaper"));
        }
        if let Some(wallpaper) = &self.wallpaper {
            return Ok(Source::Wallpaper(
                wallpaper.seed,
                wallpaper.request(self.width, self.height)?,
            ));
        }
        if self.fractal.is_some() && self.random.is_none() {
            return Err(Failure::bad("fractal goes with random: the seed of the random fractal"));
        }
        if let Some(seed) = self.random {
            return Ok(Source::Random(seed, kind(self.fractal)?));
        }
        let scene = match (self.preset, self.scene) {
            (Some(name), _) => preset(&name)?,
            (_, Some(scene)) => {
                let scene: Scene =
                    serde_json::from_value(scene).map_err(|e| Failure::bad(format!("not a scene: {e}")))?;
                scene.validate().map_err(|e| Failure::bad(e.to_string()))?;
                scene
            }
            _ => unreachable!("one source is given"),
        };
        Ok(Source::Scene(Box::new(scene)))
    }
}

async fn list_presets() -> Json<Value> {
    Json(json!({ "presets": presets::names() }))
}

async fn get_preset(Path(name): Path<String>) -> Result<Json<Scene>, Failure> {
    preset(&name).map(Json)
}

async fn list_fractals() -> Json<Value> {
    Json(json!({ "fractals": fractique_mcp::fractals() }))
}

async fn random(body: Bytes) -> Result<Json<Scene>, Failure> {
    let RandomBody { seed, fractal } = parse(&body)?;
    let fractal = kind(fractal)?;
    let scene = tokio::task::spawn_blocking(move || random_of(fractal.as_deref(), seed)).await;
    scene.map(Json).map_err(Failure::internal)
}

async fn wallpaper(body: Bytes) -> Result<Json<Scene>, Failure> {
    let body: WallpaperBody = parse(&body)?;
    // The size the command line tool lays a wallpaper out for when none is given.
    let request = body.request(1290, 2796)?;
    let scene = tokio::task::spawn_blocking(move || random_wallpaper(body.seed, &request)).await;
    scene.map(Json).map_err(Failure::internal)
}

fn develop(
    source: Source,
    seed: Option<u64>,
    options: &RenderOptions,
    cancel: &AtomicBool,
) -> Result<Response, Failure> {
    let mut scene = match source {
        Source::Scene(scene) => *scene,
        Source::Random(seed, kind) => random_of(kind.as_deref(), seed),
        Source::Wallpaper(seed, request) => random_wallpaper(seed, &request),
    };
    if let Some(seed) = seed {
        scene.seed = seed;
    }
    let control = Control {
        cancel: Some(cancel),
        progress: None,
    };
    let image = render(&scene, options, &control).map_err(|e| match e {
        RenderError::Cancelled => Failure(StatusCode::SERVICE_UNAVAILABLE, e.to_string()),
        _ => Failure::bad(e.to_string()),
    })?;
    let png = encode_png(&image).map_err(Failure::internal)?;
    let hash = format!("{:016x}", image.hash());
    Ok(([(CONTENT_TYPE, "image/png".to_owned()), (HASH_HEADER, hash)], png).into_response())
}

async fn render_png(State(shared): State<Arc<Shared>>, body: Bytes) -> Result<Response, Failure> {
    let body: RenderBody = parse(&body)?;
    let options = body.options(&shared.config)?;
    let seed = body.seed;
    let source = body.source()?;

    let ticket = Ticket::take(&shared).ok_or_else(|| {
        Failure(
            StatusCode::TOO_MANY_REQUESTS,
            "the render queue is full; try again later".into(),
        )
    })?;
    let slot: OwnedSemaphorePermit = shared.slots.clone().acquire_owned().await.map_err(Failure::internal)?;

    let cancel = Arc::new(AtomicBool::new(false));
    let _cancel_when_dropped = CancelOnDrop(cancel.clone());
    let job = tokio::task::spawn_blocking(move || {
        let _held_until_the_render_ends = (slot, ticket);
        develop(source, seed, &options, &cancel)
    });
    match tokio::time::timeout(shared.config.timeout, job).await {
        Ok(done) => done.map_err(Failure::internal)?,
        Err(_) => Err(Failure(
            StatusCode::SERVICE_UNAVAILABLE,
            format!(
                "the render took longer than the {} s this server allows",
                shared.config.timeout.as_secs_f64()
            ),
        )),
    }
}

async fn limits(State(shared): State<Arc<Shared>>) -> Json<Value> {
    let config = &shared.config;
    Json(json!({
        "jobs": config.jobs,
        "queue": config.queue,
        "max_side": config.max_side,
        "max_pixels": config.max_pixels,
        "max_samples": config.max_samples,
        "max_supersample": config.max_supersample,
        "timeout_seconds": config.timeout.as_secs_f64(),
        "max_body_bytes": MAX_BODY_BYTES,
        "mcp": config.mcp,
        "qualities": Quality::ALL.map(|(name, _)| name),
        "layouts": Layout::ALL.map(|(name, _)| name),
        "materials": Material::ALL.map(|(name, _)| name),
    }))
}

async fn health(State(shared): State<Arc<Shared>>) -> Json<Value> {
    Json(json!({ "status": "ok", "busy": *shared.busy.borrow() }))
}

async fn not_found() -> Failure {
    Failure(StatusCode::NOT_FOUND, "there is nothing at this path".into())
}

pub struct App {
    pub router: Router,
    pub busy: watch::Receiver<usize>,
}

pub fn app(config: Config) -> App {
    let (busy, watching) = watch::channel(0);
    let web = config.web.clone();
    let shared = Arc::new(Shared {
        slots: Arc::new(Semaphore::new(config.jobs.max(1))),
        host: Host {
            protocols: &mcp::PROTOCOLS,
            files: false,
            limits: Some(config.limits()),
        },
        sessions: Mutex::default(),
        config,
        busy,
    });
    let mut router = Router::new()
        .route("/presets", get(list_presets))
        .route("/presets/{name}", get(get_preset))
        .route("/random", post(random))
        .route("/fractals", get(list_fractals))
        .route("/wallpaper", post(wallpaper))
        .route("/render", post(render_png))
        .route("/limits", get(limits))
        .route("/health", get(health));
    if shared.config.mcp {
        router = router.route("/mcp", post(mcp::post).delete(mcp::delete));
    }
    let router = router.layer(DefaultBodyLimit::max(MAX_BODY_BYTES)).with_state(shared);
    let router = match web {
        Some(folder) => router.fallback_service(ServeDir::new(folder).precompressed_gzip()),
        None => router.fallback(not_found),
    };
    App { router, busy: watching }
}
