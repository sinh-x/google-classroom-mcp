//! CLI subcommand definitions and dispatch logic.
//!
//! Connects the Clap CLI to the client modules via OutputWriter.

use clap::Subcommand;
use std::path::PathBuf;

use crate::auth::{build_hubs_for_profile, discover_profiles, profile_dir_for};
use crate::calendar::CalendarClient;
use crate::classroom::ClassroomClient;
use crate::drive::DriveClient;
use crate::error::AppError;
use crate::output::{Frontmatter, OutputWriter};

#[derive(Subcommand)]
pub enum ClassroomCmd {
    /// List all courses
    Courses,
    /// Get course details and announcements
    Details { course_id: String },
    /// Get course assignments and submissions
    Assignments { course_id: String },
    /// Get course materials
    Materials { course_id: String },
    /// Get course topics/modules
    Topics { course_id: String },
}

#[derive(Subcommand)]
pub enum CalendarCmd {
    /// List all calendars
    List,
    /// List upcoming events
    Events {
        /// Calendar ID (use "primary" for the primary calendar)
        calendar_id: String,
        /// Number of days ahead to look (default: 7)
        #[arg(long, default_value = "7")]
        days_ahead: u32,
    },
    /// Get event details
    Details {
        calendar_id: String,
        event_id: String,
    },
}

#[derive(Subcommand)]
pub enum DriveCmd {
    /// Read a file by ID or URL
    Read {
        /// File ID or Google Drive URL
        file_id_or_url: String,
    },
}

/// Default output directory: ~/.local/share/personal-google-mcp/{profile}/
fn default_output_dir(profile: &str) -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("personal-google-mcp")
        .join(profile)
}

/// Verify that tokens exist for the given profile (fail-fast auth check).
fn verify_auth(profile: &str) -> Result<(), AppError> {
    let dir = profile_dir_for(profile)?;
    let tokens_path = dir.join("tokens.json");
    if !tokens_path.exists() {
        return Err(AppError::CredentialRead(format!(
            "not authenticated for profile '{profile}' — run `personal-google-mcp auth` first"
        )));
    }
    Ok(())
}

/// Resolve the output directory from CLI args or default.
fn resolve_output_dir(profile: &str, output_dir: &Option<PathBuf>) -> PathBuf {
    output_dir.clone().unwrap_or_else(|| default_output_dir(profile))
}

/// Run the profiles subcommand — lists profiles to stdout, no file output.
pub fn run_profiles() -> Result<(), AppError> {
    let profiles = discover_profiles()?;
    for (name, _dir) in &profiles {
        println!("{}", name);
    }
    Ok(())
}

/// Run a classroom subcommand.
pub async fn run_classroom(
    cmd: &ClassroomCmd,
    profile: &str,
    output_dir: &Option<PathBuf>,
) -> Result<(), AppError> {
    verify_auth(profile)?;
    let dir = profile_dir_for(profile)?;
    let hubs = build_hubs_for_profile(profile, &dir).await?;
    let cache_dir = profile_dir_for(profile)?.join("cache");
    let writer = OutputWriter::new(profile.to_string(), resolve_output_dir(profile, output_dir));

    match cmd {
        ClassroomCmd::Courses => {
            let client = ClassroomClient::new(hubs.classroom, cache_dir);
            let data = client.list_courses().await?;
            let fm = Frontmatter {
                tool: "classroom/courses".to_string(),
                profile: profile.to_string(),
                date: chrono::Utc::now().to_rfc3339(),
                params: None,
            };
            let path = writer.write_output("courses", &data, &fm)?;
            println!("{}", path.display());
        }
        ClassroomCmd::Details { course_id } => {
            let client = ClassroomClient::new(hubs.classroom, cache_dir);
            let data = client.get_course_details(course_id).await?;
            let fm = Frontmatter {
                tool: "classroom/details".to_string(),
                profile: profile.to_string(),
                date: chrono::Utc::now().to_rfc3339(),
                params: Some(serde_json::json!({ "course_id": course_id })),
            };
            let slug = format!("details-{}", course_id);
            let path = writer.write_output(&slug, &data, &fm)?;
            println!("{}", path.display());
        }
        ClassroomCmd::Assignments { course_id } => {
            let client = ClassroomClient::new(hubs.classroom, cache_dir);
            let data = client.get_assignments(course_id).await?;
            let fm = Frontmatter {
                tool: "classroom/assignments".to_string(),
                profile: profile.to_string(),
                date: chrono::Utc::now().to_rfc3339(),
                params: Some(serde_json::json!({ "course_id": course_id })),
            };
            let slug = format!("assignments-{}", course_id);
            let path = writer.write_output(&slug, &data, &fm)?;
            println!("{}", path.display());
        }
        ClassroomCmd::Materials { course_id } => {
            let client = ClassroomClient::new(hubs.classroom, cache_dir);
            let data = client.get_course_materials(course_id).await?;
            let fm = Frontmatter {
                tool: "classroom/materials".to_string(),
                profile: profile.to_string(),
                date: chrono::Utc::now().to_rfc3339(),
                params: Some(serde_json::json!({ "course_id": course_id })),
            };
            let slug = format!("materials-{}", course_id);
            let path = writer.write_output(&slug, &data, &fm)?;
            println!("{}", path.display());
        }
        ClassroomCmd::Topics { course_id } => {
            let client = ClassroomClient::new(hubs.classroom, cache_dir);
            let data = client.get_course_topics(course_id).await?;
            let fm = Frontmatter {
                tool: "classroom/topics".to_string(),
                profile: profile.to_string(),
                date: chrono::Utc::now().to_rfc3339(),
                params: Some(serde_json::json!({ "course_id": course_id })),
            };
            let slug = format!("topics-{}", course_id);
            let path = writer.write_output(&slug, &data, &fm)?;
            println!("{}", path.display());
        }
    }
    Ok(())
}

/// Run a calendar subcommand.
pub async fn run_calendar(
    cmd: &CalendarCmd,
    profile: &str,
    output_dir: &Option<PathBuf>,
) -> Result<(), AppError> {
    verify_auth(profile)?;
    let dir = profile_dir_for(profile)?;
    let hubs = build_hubs_for_profile(profile, &dir).await?;
    let writer = OutputWriter::new(profile.to_string(), resolve_output_dir(profile, output_dir));

    match cmd {
        CalendarCmd::List => {
            let client = CalendarClient::new(hubs.calendar);
            let data = client.list_calendars().await?;
            let fm = Frontmatter {
                tool: "calendar/calendars".to_string(),
                profile: profile.to_string(),
                date: chrono::Utc::now().to_rfc3339(),
                params: None,
            };
            let path = writer.write_output("calendars", &data, &fm)?;
            println!("{}", path.display());
        }
        CalendarCmd::Events { calendar_id, days_ahead } => {
            let client = CalendarClient::new(hubs.calendar);
            let data = client.list_events(calendar_id, *days_ahead).await?;
            let fm = Frontmatter {
                tool: "calendar/events".to_string(),
                profile: profile.to_string(),
                date: chrono::Utc::now().to_rfc3339(),
                params: Some(serde_json::json!({
                    "calendar_id": calendar_id,
                    "days_ahead": days_ahead
                })),
            };
            let slug = format!("events-{}", calendar_id);
            let path = writer.write_output(&slug, &data, &fm)?;
            println!("{}", path.display());
        }
        CalendarCmd::Details { calendar_id, event_id } => {
            let client = CalendarClient::new(hubs.calendar);
            let data = client.get_event(calendar_id, event_id).await?;
            let fm = Frontmatter {
                tool: "calendar/event-details".to_string(),
                profile: profile.to_string(),
                date: chrono::Utc::now().to_rfc3339(),
                params: Some(serde_json::json!({
                    "calendar_id": calendar_id,
                    "event_id": event_id
                })),
            };
            let slug = format!("event-{}-{}", calendar_id, event_id);
            let path = writer.write_output(&slug, &data, &fm)?;
            println!("{}", path.display());
        }
    }
    Ok(())
}

/// Run a drive subcommand.
pub async fn run_drive(
    cmd: &DriveCmd,
    profile: &str,
    output_dir: &Option<PathBuf>,
) -> Result<(), AppError> {
    verify_auth(profile)?;
    let dir = profile_dir_for(profile)?;
    let hubs = build_hubs_for_profile(profile, &dir).await?;
    let writer = OutputWriter::new(profile.to_string(), resolve_output_dir(profile, output_dir));

    match cmd {
        DriveCmd::Read { file_id_or_url } => {
            let client = DriveClient::new(hubs.drive);
            let data = client.read_material(file_id_or_url).await?;
            let fm = Frontmatter {
                tool: "drive/read".to_string(),
                profile: profile.to_string(),
                date: chrono::Utc::now().to_rfc3339(),
                params: Some(serde_json::json!({ "file_id_or_url": file_id_or_url })),
            };
            // Use a slugified version of the filename from metadata if available
            let name = data.get("metadata")
                .and_then(|m| m.get("name"))
                .and_then(|v| v.as_str())
                .map(OutputWriter::slugify)
                .unwrap_or_else(|| "file".to_string());
            let path = writer.write_output(&name, &data, &fm)?;
            println!("{}", path.display());
        }
    }
    Ok(())
}