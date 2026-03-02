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

#[derive(Debug, Clone)]
pub struct GoogleService {
    client: Arc<ClassroomClient>,
    drive_client: Arc<DriveClient>,
    calendar_client: Arc<CalendarClient>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CourseIdParam {
    #[schemars(description = "The ID of the course")]
    pub course_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CalendarEventsParam {
    #[schemars(description = "Calendar ID (use 'primary' for the user's main calendar)")]
    pub calendar_id: String,
    #[schemars(description = "Number of days ahead to fetch events (default: 7)")]
    pub days_ahead: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CalendarEventDetailParam {
    #[schemars(description = "Calendar ID (use 'primary' for the user's main calendar)")]
    pub calendar_id: String,
    #[schemars(description = "The event ID")]
    pub event_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadMaterialParam {
    #[schemars(
        description = "A Google Drive file ID or full URL (e.g. https://docs.google.com/document/d/FILE_ID/edit)"
    )]
    pub file_id_or_url: String,
}

#[tool_router]
impl GoogleService {
    pub fn new(
        client: Arc<ClassroomClient>,
        drive_client: Arc<DriveClient>,
        calendar_client: Arc<CalendarClient>,
    ) -> Self {
        Self {
            client,
            drive_client,
            calendar_client,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "List all Google Classroom courses for the authenticated user")]
    async fn courses(&self) -> String {
        match self.client.list_courses().await {
            Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        description = "Get details for a specific course including recent announcements (up to 20)"
    )]
    async fn course_details(&self, Parameters(params): Parameters<CourseIdParam>) -> String {
        match self.client.get_course_details(&params.course_id).await {
            Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        description = "Get assignments (coursework) for a course with student submissions for the first 5 assignments"
    )]
    async fn assignments(&self, Parameters(params): Parameters<CourseIdParam>) -> String {
        match self.client.get_assignments(&params.course_id).await {
            Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        description = "Get course work materials (posted resources like documents, links, videos) for a course"
    )]
    async fn course_materials(&self, Parameters(params): Parameters<CourseIdParam>) -> String {
        match self.client.get_course_materials(&params.course_id).await {
            Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        description = "Get topics (modules/sections) for a course that organize coursework and materials"
    )]
    async fn course_topics(&self, Parameters(params): Parameters<CourseIdParam>) -> String {
        match self.client.get_course_topics(&params.course_id).await {
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
        match self.drive_client.read_material(&params.file_id_or_url).await {
            Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "List all Google Calendars the authenticated user has access to")]
    async fn calendars(&self) -> String {
        match self.calendar_client.list_calendars().await {
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
        let days = params.days_ahead.unwrap_or(7);
        match self.calendar_client.list_events(&params.calendar_id, days).await {
            Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(description = "Get full details for a specific Google Calendar event")]
    async fn calendar_event_details(
        &self,
        Parameters(params): Parameters<CalendarEventDetailParam>,
    ) -> String {
        match self.calendar_client.get_event(&params.calendar_id, &params.event_id).await {
            Ok(val) => serde_json::to_string_pretty(&val).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {e}"),
        }
    }
}

#[tool_handler]
impl ServerHandler for GoogleService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Personal Google MCP server — provides access to Google services including \
                 Classroom (courses, announcements, assignments, materials), \
                 Calendar (list calendars, upcoming events, event details), \
                 Drive (file reading), and more services coming soon (Gmail, etc.)."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
