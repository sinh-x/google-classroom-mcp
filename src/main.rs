mod auth;
mod classroom;
mod error;
mod tools;

use std::sync::Arc;

use clap::{Parser, Subcommand};
use rmcp::ServiceExt;
use rmcp::transport::stdio;

use crate::auth::{build_hub, run_auth_flow};
use crate::classroom::ClassroomClient;
use crate::tools::ClassroomService;

#[derive(Parser)]
#[command(name = "google-classroom-mcp", about = "MCP server for Google Classroom")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Start the MCP server (default)
    Run,
    /// Authenticate with Google and save tokens
    Auth,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Tracing to stderr — stdout is reserved for MCP stdio transport
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    match cli.command.unwrap_or(Command::Run) {
        Command::Auth => {
            run_auth_flow().await?;
        }
        Command::Run => {
            let hub = build_hub().await?;
            let client = Arc::new(ClassroomClient::new(hub));
            let service = ClassroomService::new(client);

            tracing::info!("Starting MCP server on stdio...");
            let server = service.serve(stdio()).await?;
            server.waiting().await?;
        }
    }

    Ok(())
}
