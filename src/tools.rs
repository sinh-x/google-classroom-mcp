use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::classroom::ClassroomClient;
use crate::drive::DriveClient;

#[derive(Debug, Clone)]
pub struct ClassroomService {
    client: Arc<ClassroomClient>,
    drive_client: Arc<DriveClient>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CourseIdParam {
    #[schemars(description = "The ID of the course")]
    pub course_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadMaterialParam {
    #[schemars(
        description = "A Google Drive file ID or full URL (e.g. https://docs.google.com/document/d/FILE_ID/edit)"
    )]
    pub file_id_or_url: String,
}

#[tool_router]
impl ClassroomService {
    pub fn new(client: Arc<ClassroomClient>, drive_client: Arc<DriveClient>) -> Self {
        Self {
            client,
            drive_client,
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
}

#[tool_handler]
impl ServerHandler for ClassroomService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Google Classroom MCP server — provides read-only access to courses, \
                 announcements, assignments, student submissions, course materials, and topics. \
                 Can also read Google Drive file contents (Docs, Sheets, Slides, text files)."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
