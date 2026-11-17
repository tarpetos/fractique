mod mcp;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use clap::{Parser, Subcommand};
use fractique_core::import::import_flames;
use fractique_core::random::{fit_camera, mutate, random_scene};
use fractique_core::{Control, Image, Kind, Quality, RenderOptions, Scene, presets, render};

#[derive(Parser)]
#[command(name = "fractique", version, about = "Deterministic fractal flame generator")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Render a scene to a PNG
    Render {
        /// Scene file (JSON); or use --preset or --random
        scene: Option<PathBuf>,
        #[arg(long, conflicts_with = "scene")]
        preset: Option<String>,
        /// Render the random scene of this seed
        #[arg(long, conflicts_with_all = ["scene", "preset"])]
        random: Option<u64>,
        #[arg(short, long, default_value = "fractique.png")]
        output: PathBuf,
        /// WIDTHxHEIGHT
        #[arg(long, default_value = "1920x1080")]
        size: String,
        /// draft, mobile, standard, high or ultra
        #[arg(long, default_value = "standard")]
        quality: String,
        /// Samples per pixel, instead of the quality's
        #[arg(long)]
        samples: Option<u32>,
        /// Supersample (1 to 4), instead of the quality's
        #[arg(long)]
        supersample: Option<u32>,
        /// Replace the scene's seed
        #[arg(long)]
        seed: Option<u64>,
    },
    /// List the built-in presets
    Presets,
    /// Print a built-in preset as JSON
    Preset { name: String },
    /// Print the random scene of a seed as JSON
    Randomize {
        seed: u64,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Print a variant of a scene as JSON
    Mutate {
        scene: PathBuf,
        seed: u64,
        /// 0 to 1
        #[arg(long, default_value_t = 0.3)]
        strength: f64,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Print a scene with every layer's camera moved to frame its flame
    Fit {
        scene: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Convert a .flame file (flam3, Apophysis) to a scene
    Import {
        file: PathBuf,
        /// Which flame of the file, counted from 0
        #[arg(long, default_value_t = 0)]
        index: usize,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// List the variations and their parameters
    Variations,
    /// Run as an MCP server on stdin and stdout, for AI agents
    Mcp,
}

fn parse_size(text: &str) -> Result<(u32, u32), String> {
    let parsed = text
        .split_once(['x', 'X'])
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)));
    parsed.ok_or_else(|| format!("'{text}' is not a size; write it as 1920x1080"))
}

fn load(path: &Path) -> Result<Scene, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Scene::from_json(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn emit(scene: &Scene, output: Option<PathBuf>) -> Result<(), String> {
    match output {
        Some(path) => fs::write(&path, scene.to_json() + "\n").map_err(|e| format!("{}: {e}", path.display())),
        None => {
            println!("{}", scene.to_json());
            Ok(())
        }
    }
}

pub(crate) fn encode_png(image: &Image) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(&image.rgba).map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())?;
    Ok(bytes)
}

fn save_png(image: &Image, path: &Path) -> Result<(), String> {
    fs::write(path, encode_png(image)?).map_err(|e| format!("{}: {e}", path.display()))
}

fn run(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Render {
            scene,
            preset,
            random,
            output,
            size,
            quality,
            samples,
            supersample,
            seed,
        } => {
            let mut scene = match (scene, preset, random) {
                (Some(path), _, _) => load(&path)?,
                (_, Some(name), _) => presets::get(&name).ok_or_else(|| format!("no preset is called '{name}'"))?,
                (_, _, Some(seed)) => random_scene(seed),
                _ => return Err("give a scene file, --preset NAME or --random SEED".into()),
            };
            if let Some(seed) = seed {
                scene.seed = seed;
            }
            let (width, height) = parse_size(&size)?;
            let quality = Quality::parse(&quality).ok_or_else(|| format!("no quality is called '{quality}'"))?;
            let mut options = RenderOptions::new(width, height, quality);
            options.samples_per_pixel = samples.unwrap_or(options.samples_per_pixel);
            options.supersample = supersample.unwrap_or(options.supersample);

            let shown = AtomicU64::new(0);
            let progress = |done: f64| {
                let percent = (done * 100.0) as u64;
                if shown.fetch_max(percent, Ordering::Relaxed) < percent {
                    eprint!("\r{percent:3}%");
                    let _ = std::io::stderr().flush();
                }
            };
            let started = Instant::now();
            let image = render(
                &scene,
                &options,
                &Control {
                    cancel: None,
                    progress: Some(&progress),
                },
            )
            .map_err(|e| e.to_string())?;
            save_png(&image, &output)?;
            eprintln!(
                "\r{} ({width}x{height}, {} samples per pixel, {:.1} s, hash {:016x})",
                output.display(),
                options.samples_per_pixel,
                started.elapsed().as_secs_f64(),
                image.hash()
            );
            Ok(())
        }
        Command::Presets => {
            presets::names().iter().for_each(|name| println!("{name}"));
            Ok(())
        }
        Command::Preset { name } => {
            let scene = presets::get(&name).ok_or_else(|| format!("no preset is called '{name}'"))?;
            emit(&scene, None)
        }
        Command::Randomize { seed, output } => emit(&random_scene(seed), output),
        Command::Mutate {
            scene,
            seed,
            strength,
            output,
        } => emit(&mutate(&load(&scene)?, seed, strength), output),
        Command::Fit { scene, output } => {
            let mut scene = load(&scene)?;
            for (index, layer) in scene.layers.iter_mut().enumerate() {
                let (camera, _) = fit_camera(&layer.flame)
                    .ok_or_else(|| format!("layer {index}: the flame draws nothing to frame"))?;
                layer.flame.camera = camera;
            }
            emit(&scene, output)
        }
        Command::Import { file, index, output } => {
            let text = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            let mut flames = import_flames(&text).map_err(|e| format!("{}: {e}", file.display()))?;
            let count = flames.len();
            if index >= count {
                return Err(format!(
                    "{} holds {count} flames; --index goes from 0 to {}",
                    file.display(),
                    count - 1
                ));
            }
            let flame = flames.swap_remove(index);
            for warning in &flame.warnings {
                eprintln!("fractique: {warning}");
            }
            if count > 1 {
                eprintln!("fractique: flame {index} of {count}; choose another with --index");
            }
            emit(&flame.scene, output)
        }
        Command::Mcp => mcp::serve(),
        Command::Variations => {
            for kind in Kind::ALL {
                let params: Vec<String> = kind
                    .params()
                    .iter()
                    .map(|(name, default)| format!("{name}={default}"))
                    .collect();
                println!("{} {}", kind.name(), params.join(" "));
            }
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("fractique: {message}");
            ExitCode::FAILURE
        }
    }
}
