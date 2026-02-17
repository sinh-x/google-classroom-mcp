use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::*;
use rmcp::{tool, tool_handler, tool_router, ServerHandler};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::classroom::ClassroomClient;

#[derive(Debug, Clone)]
pub struct ClassroomService {
    client: Arc<ClassroomClient>,
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CourseIdParam {
    #[schemars(description = "The ID of the course")]
    pub course_id: String,
}

#[tool_router]
impl ClassroomService {
    pub fn new(client: Arc<ClassroomClient>) -> Self {
        Self {
            client,
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
}

#[tool_handler]
impl ServerHandler for ClassroomService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Google Classroom MCP server — provides read-only access to courses, \
                 announcements, assignments, and student submissions."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
