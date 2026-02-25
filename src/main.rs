mod auth;
mod classroom;
mod drive;
mod error;
mod tools;

use std::sync::Arc;

use clap::{Parser, Subcommand};
use rmcp::ServiceExt;
use rmcp::transport::stdio;

use crate::auth::{build_hubs, run_auth_flow};
use crate::classroom::ClassroomClient;
use crate::drive::DriveClient;
use crate::tools::GoogleService;

#[derive(Parser)]
#[command(name = "personal-google-mcp", about = "MCP server for personal Google services")]
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
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

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
            let (classroom_hub, drive_hub) = build_hubs().await?;
            let client = Arc::new(ClassroomClient::new(classroom_hub));
            let drive_client = Arc::new(DriveClient::new(drive_hub));
            let service = GoogleService::new(client, drive_client);

            tracing::info!("Starting MCP server on stdio...");
            let server = service.serve(stdio()).await?;
            server.waiting().await?;
        }
    }

    Ok(())
}
