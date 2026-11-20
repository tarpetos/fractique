mod mcp;

use std::process::ExitCode;

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "fractique-mcp",
    version,
    about = "Fractique as an MCP server on stdin and stdout, for AI agents"
)]
struct Cli {}

fn main() -> ExitCode {
    let Cli {} = Cli::parse();
    match mcp::serve() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("fractique-mcp: {message}");
            ExitCode::FAILURE
        }
    }
}
