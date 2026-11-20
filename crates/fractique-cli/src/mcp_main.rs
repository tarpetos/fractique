mod mcp;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "fractique-mcp",
    version,
    about = "Fractique as an MCP server on stdin and stdout, for AI agents"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Replace this program with the newest release
    Update(fractique_update::Options),
}

fn main() -> ExitCode {
    if let Some(answered) = fractique_update::start(fractique_update::MCP) {
        return answered;
    }
    if let Some(Command::Update(options)) = Cli::parse().command {
        return fractique_update::update(fractique_update::MCP, &options);
    }
    match mcp::serve() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("fractique-mcp: {message}");
            ExitCode::FAILURE
        }
    }
}
