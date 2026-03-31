use std::collections::HashMap;
use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::calendar::CalendarClient;
use crate::classroom::ClassroomClient;
use crate::drive::DriveClient;

/// Clients for a single authenticated profile.
#[derive(Debug, Clone)]
pub struct ProfileClients {
    pub classroom: Arc<ClassroomClient>,
    pub drive: Arc<DriveClient>,
    pub calendar: Arc<CalendarClient>,
}

#[derive(Debug, Clone)]
pub struct GoogleService {
    profiles: HashMap<String, ProfileClients>,
    default_profile: String,
    tool_router: ToolRouter<Self>,
}

impl GoogleService {
    fn resolve_profile(&self, profile: Option<&str>) -> Result<&ProfileClients, String> {
        let name = profile.unwrap_or(&self.default_profile);
        self.profiles.get(name).ok_or_else(|| {
            let available: Vec<&str> = self.profiles.keys().map(|s| s.as_str()).collect();
            format!(
                "profile '{}' not found. Available profiles: {}",
                name,
                available.join(", ")
            )
        })
    }
}

// --- Tool parameter structs ---

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ProfileParam {
    #[schemars(description = "Profile name to use (omit for default profile). Use list_profiles to see available profiles.")]
    pub profile: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CourseIdParam {
    #[schemars(description = "The ID of the course")]
    pub course_id: String,
    #[schemars(description = "Profile name to use (omit for default profile)")]
    pub profile: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CalendarEventsParam {
    #[schemars(description = "Calendar ID (use 'primary' for the user's main calendar)")]
    pub calendar_id: String,
    #[schemars(description = "Number of days ahead to fetch events (default: 7)")]
    pub days_ahead: Option<u32>,
    #[schemars(description = "Profile name to use (omit for default profile)")]
    pub profile: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CalendarEventDetailParam {
    #[schemars(description = "Calendar ID (use 'primary' for the user's main calendar)")]
    pub calendar_id: String,
    #[schemars(description = "The event ID")]
    pub event_id: String,
    #[schemars(description = "Profile name to use (omit for default profile)")]
    pub profile: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadMaterialParam {
    #[schemars(
        description = "A Google Drive file ID or full URL (e.g. https://docs.google.com/document/d/FILE_ID/edit)"
    )]
    pub file_id_or_url: String,
    #[schemars(description = "Profile name to use (omit for default profile)")]
    pub profile: Option<String>,
}

#[tool_router]
impl GoogleService {
    pub fn new(profiles: HashMap<String, ProfileClients>) -> Self {
        // Pick default: prefer "default", otherwise first alphabetically
        let default_profile = if profiles.contains_key("default") {
            "default".to_string()
        } else {
            let mut keys: Vec<&String> = profiles.keys().collect();
            keys.sort();
            keys.first()
                .map(|k| k.to_string())
                .unwrap_or_else(|| "default".to_string())
        };

        Self {
            profiles,
            default_profile,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "List available Google account profiles and which is the default")]
    async fn list_profiles(&self) -> String {
        let mut profile_names: Vec<&String> = self.profiles.keys().collect();
        profile_names.sort();
        let profiles: Vec<serde_json::Value> = profile_names
            .iter()
            .map(|name| {
                serde_json::json!({
                    "name": name,
                    "is_default": **name == self.default_profile,
                })
            })
            .collect();
        serde_json::to_string_pretty(&profiles).unwrap_or_else(|e| e.to_string())
    }

    #[tool(description = "List all Google Classroom courses for the authenticated user")]
    async fn courses(&self, Parameters(params): Parameters<ProfileParam>) -> String {
        let clients = match self.resolve_profile(params.profile.as_deref()) {
            Ok(c) => c,
            Err(e) => return format!("Error: {e}"),
        };
        match clients.classroom.list_courses().await {
            Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        description = "Get details for a specific course including recent announcements (up to 20)"
    )]
    async fn course_details(&self, Parameters(params): Parameters<CourseIdParam>) -> String {
        let clients = match self.resolve_profile(params.profile.as_deref()) {
            Ok(c) => c,
            Err(e) => return format!("Error: {e}"),
        };
        match clients.classroom.get_course_details(&params.course_id).await {
            Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        description = "Get assignments (coursework) for a course with student submissions for the first 5 assignments"
    )]
    async fn assignments(&self, Parameters(params): Parameters<CourseIdParam>) -> String {
        let clients = match self.resolve_profile(params.profile.as_deref()) {
            Ok(c) => c,
            Err(e) => return format!("Error: {e}"),
        };
        match clients.classroom.get_assignments(&params.course_id).await {
            Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        description = "Get course work materials (posted resources like documents, links, videos) for a course"
    )]
    async fn course_materials(&self, Parameters(params): Parameters<CourseIdParam>) -> String {
        let clients = match self.resolve_profile(params.profile.as_deref()) {
            Ok(c) => c,
            Err(e) => return format!("Error: {e}"),
        };
        match clients.classroom.get_course_materials(&params.course_id).await {
            Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        description = "Get topics (modules/sections) for a course that organize coursework and materials"
    )]
    async fn course_topics(&self, Parameters(params): Parameters<CourseIdParam>) -> String {
        let clients = match self.resolve_profile(params.profile.as_deref()) {
            Ok(c) => c,
            Err(e) => return format!("Error: {e}"),
        };
        match clients.classroom.get_course_topics(&params.course_id).await {
            Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        description = "Read the content of a Google Drive file (Docs, Sheets, Slides, or plain text). \
                        Accepts a file ID or full Google Drive/Docs URL. \
                        Google Workspace documents are exported to text; binary files return metadata only."
    )]
    async fn read_material(
        &self,
        Parameters(params): Parameters<ReadMaterialParam>,
    ) -> String {
        let clients = match self.resolve_profile(params.profile.as_deref()) {
            Ok(c) => c,
            Err(e) => return format!("Error: {e}"),
        };
        match clients.drive.read_material(&params.file_id_or_url).await {
            Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all Google Calendars the authenticated user has access to")]
    async fn calendars(&self, Parameters(params): Parameters<ProfileParam>) -> String {
        let clients = match self.resolve_profile(params.profile.as_deref()) {
            Ok(c) => c,
            Err(e) => return format!("Error: {e}"),
        };
        match clients.calendar.list_calendars().await {
            Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        description = "List upcoming events on a Google Calendar. \
                        Use calendar_id 'primary' for the user's main calendar."
    )]
    async fn calendar_events(
        &self,
        Parameters(params): Parameters<CalendarEventsParam>,
    ) -> String {
        let clients = match self.resolve_profile(params.profile.as_deref()) {
            Ok(c) => c,
            Err(e) => return format!("Error: {e}"),
        };
        let days = params.days_ahead.unwrap_or(7);
        match clients.calendar.list_events(&params.calendar_id, days).await {
            Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Get full details for a specific Google Calendar event")]
    async fn calendar_event_details(
        &self,
        Parameters(params): Parameters<CalendarEventDetailParam>,
    ) -> String {
        let clients = match self.resolve_profile(params.profile.as_deref()) {
            Ok(c) => c,
            Err(e) => return format!("Error: {e}"),
        };
        match clients.calendar.get_event(&params.calendar_id, &params.event_id).await {
            Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {e}"),
        }
    }
}

#[tool_handler]
impl ServerHandler for GoogleService {
    fn get_info(&self) -> ServerInfo {
        let mut profile_names: Vec<&String> = self.profiles.keys().collect();
        profile_names.sort();
        let profiles_list = profile_names
            .iter()
            .map(|n| {
                if **n == self.default_profile {
                    format!("{n} (default)")
                } else {
                    n.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");

        let instructions = format!(
            "Personal Google MCP server — provides access to Google services including \
             Classroom (courses, announcements, assignments, materials), \
             Calendar (list calendars, upcoming events, event details), \
             Drive (file reading), and more services coming soon (Gmail, etc.). \
             Available profiles: {profiles_list}. \
             Use the 'profile' parameter on any tool to select a specific account."
        );

        ServerInfo {
            instructions: Some(instructions.into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
