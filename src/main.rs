mod auth;
mod calendar;
mod classroom;
mod cli;
mod drive;
mod error;
mod formatters;
mod output;
mod tools;

use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use rmcp::ServiceExt;
use rmcp::transport::stdio;

use crate::auth::{active_profile, build_all_hubs, profile_dir_for, run_auth_flow};
use crate::calendar::CalendarClient;
use crate::classroom::ClassroomClient;
use crate::cli::{
    run_calendar, run_classroom, run_drive, run_profiles,
    CalendarCmd, ClassroomCmd, DriveCmd,
};
use crate::drive::DriveClient;
use crate::tools::{GoogleService, ProfileClients};

#[derive(Parser)]
#[command(name = "personal-google-mcp", version, about = "MCP server for personal Google services")]
struct Cli {
    /// Profile name to use (default: "default")
    #[arg(long, global = true, default_value = "default")]
    profile: String,

    /// Override the output directory (default: ~/.local/share/personal-google-mcp/{profile}/)
    #[arg(long, global = true)]
    output_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Start the MCP server (default)
    Run,
    /// Authenticate with Google and save tokens
    Auth,
    /// Classroom subcommands
    Classroom {
        #[command(subcommand)]
        cmd: ClassroomCmd,
    },
    /// Calendar subcommands
    Calendar {
        #[command(subcommand)]
        cmd: CalendarCmd,
    },
    /// Drive subcommands
    Drive {
        #[command(subcommand)]
        cmd: DriveCmd,
    },
    /// List available profiles
    Profiles,
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
            if let Some(profile) = active_profile() {
                tracing::info!("Active profile: {profile}");
            }
            run_auth_flow().await?;
        }
        Command::Run => {
            let all_hubs = build_all_hubs().await?;
            let mut profiles = std::collections::HashMap::new();
            for (name, hubs) in all_hubs {
                let cache_dir = profile_dir_for(&name)?.join("cache");
                let classroom = Arc::new(ClassroomClient::new(hubs.classroom, cache_dir));
                let drive = Arc::new(DriveClient::new(hubs.drive));
                let calendar = Arc::new(CalendarClient::new(hubs.calendar));
                profiles.insert(name, ProfileClients { classroom, drive, calendar });
            }
            let service = GoogleService::new(profiles);

            tracing::info!("Starting MCP server on stdio...");
            let server = service.serve(stdio()).await?;
            server.waiting().await?;
        }
        Command::Profiles => {
            run_profiles()?;
        }
        Command::Classroom { cmd } => {
            run_classroom(&cmd, &cli.profile, &cli.output_dir).await?;
        }
        Command::Calendar { cmd } => {
            run_calendar(&cmd, &cli.profile, &cli.output_dir).await?;
        }
        Command::Drive { cmd } => {
            run_drive(&cmd, &cli.profile, &cli.output_dir).await?;
        }
    }

    Ok(())
}