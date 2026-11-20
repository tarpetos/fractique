use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use fractique_server::{Config, app};

#[derive(Parser)]
#[command(name = "fractique-server", version, about = "HTTP API and web UI for Fractique")]
struct Args {
    /// Address and port to listen on
    #[arg(long, env = "FRACTIQUE_BIND", default_value = "127.0.0.1:8080")]
    bind: SocketAddr,
    /// Folder with the web build of the app (crates/fractique-app/dist), served at /
    #[arg(long, env = "FRACTIQUE_WEB")]
    web: Option<PathBuf>,
    /// Renders that run at once
    #[arg(long, env = "FRACTIQUE_JOBS", default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..))]
    jobs: u32,
    /// Renders that may wait for a free slot; one more is refused with 429
    #[arg(long, env = "FRACTIQUE_QUEUE", default_value_t = 8)]
    queue: u32,
    /// Largest width and height, in pixels
    #[arg(long, env = "FRACTIQUE_MAX_SIDE", default_value_t = 4096)]
    max_side: u32,
    /// Largest width × height
    #[arg(long, env = "FRACTIQUE_MAX_PIXELS", default_value_t = 3840 * 2160)]
    max_pixels: u64,
    /// Most samples per pixel
    #[arg(long, env = "FRACTIQUE_MAX_SAMPLES", default_value_t = 1000)]
    max_samples: u32,
    /// Largest supersample
    #[arg(long, env = "FRACTIQUE_MAX_SUPERSAMPLE", default_value_t = 2)]
    max_supersample: u32,
    /// Seconds a render may take before it is stopped
    #[arg(long, env = "FRACTIQUE_TIMEOUT", default_value_t = 120)]
    timeout: u64,
    /// MCP for AI agents at /mcp: on or off
    #[arg(
        long,
        env = "FRACTIQUE_MCP",
        default_value = "on",
        value_name = "on|off",
        hide_possible_values = true,
        action = clap::ArgAction::Set,
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    mcp: bool,
    /// Origin of a web page that may call /mcp besides the pages of this machine; * for any
    #[arg(long = "mcp-origin", env = "FRACTIQUE_MCP_ORIGINS", value_delimiter = ',')]
    mcp_origins: Vec<String>,
    /// The API reference at /docs and its description at /openapi.json: on or off
    #[arg(
        long,
        env = "FRACTIQUE_DOCS",
        default_value = "on",
        value_name = "on|off",
        hide_possible_values = true,
        action = clap::ArgAction::Set,
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    docs: bool,
}

async fn stop_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        if let Ok(mut terminate) = signal(SignalKind::terminate()) {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = terminate.recv() => {}
            }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}

async fn run(args: Args) -> Result<(), String> {
    if let Some(folder) = &args.web
        && !folder.join("index.html").is_file()
    {
        return Err(format!("{}: no index.html in the web folder", folder.display()));
    }
    let config = Config {
        jobs: args.jobs as usize,
        queue: args.queue as usize,
        max_side: args.max_side,
        max_pixels: args.max_pixels,
        max_samples: args.max_samples,
        max_supersample: args.max_supersample,
        timeout: Duration::from_secs(args.timeout),
        web: args.web,
        mcp: args.mcp,
        mcp_origins: args.mcp_origins,
        docs: args.docs,
    };
    let listener = tokio::net::TcpListener::bind(args.bind)
        .await
        .map_err(|e| format!("{}: {e}", args.bind))?;
    let address = listener.local_addr().map_err(|e| e.to_string())?;
    eprintln!("fractique-server: listening on http://{address}");
    axum::serve(listener, app(config).router)
        .with_graceful_shutdown(stop_signal())
        .await
        .map_err(|e| e.to_string())
}

#[tokio::main]
async fn main() -> ExitCode {
    match run(Args::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("fractique-server: {message}");
            ExitCode::FAILURE
        }
    }
}
