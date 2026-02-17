use serde_json::{json, Value};

use crate::auth::ClassroomHub;
use crate::error::AppError;

pub struct ClassroomClient {
    hub: ClassroomHub,
}

impl std::fmt::Debug for ClassroomClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClassroomClient").finish_non_exhaustive()
    }
}

impl ClassroomClient {
    pub fn new(hub: ClassroomHub) -> Self {
        Self { hub }
    }

    /// List all courses the authenticated user can see.
    pub async fn list_courses(&self) -> Result<Value, AppError> {
        let (_resp, list) = self
            .hub
            .courses()
            .list()
            .page_size(100)
            .doit()
            .await
            .map_err(|e| AppError::GoogleApi(e.to_string()))?;

        let courses = list.courses.unwrap_or_default();
        serde_json::to_value(&courses).map_err(AppError::Json)
    }

    /// Get course details plus its most recent announcements.
    pub async fn get_course_details(&self, course_id: &str) -> Result<Value, AppError> {
        let (_resp, course) = self
            .hub
            .courses()
            .get(course_id)
            .doit()
            .await
            .map_err(|e| AppError::GoogleApi(e.to_string()))?;

        let announcements = match self
            .hub
            .courses()
            .announcements_list(course_id)
            .page_size(20)
            .doit()
            .await
        {
            Ok((_resp, list)) => {
                serde_json::to_value(list.announcements.unwrap_or_default()).unwrap_or(json!([]))
            }
            Err(e) => {
                tracing::warn!("failed to fetch announcements for {course_id}: {e}");
                json!([])
            }
        };

        Ok(json!({
            "course": serde_json::to_value(&course).map_err(AppError::Json)?,
            "announcements": announcements,
        }))
    }

    /// Get coursework for a course plus student submissions for the first 5 assignments.
    pub async fn get_assignments(&self, course_id: &str) -> Result<Value, AppError> {
        let (_resp, course) = self
            .hub
            .courses()
            .get(course_id)
            .doit()
            .await
            .map_err(|e| AppError::GoogleApi(e.to_string()))?;

        let course_work_list = match self
            .hub
            .courses()
            .course_work_list(course_id)
            .page_size(50)
            .doit()
            .await
        {
            Ok((_resp, list)) => list.course_work.unwrap_or_default(),
            Err(e) => {
                tracing::warn!("failed to fetch coursework for {course_id}: {e}");
                Vec::new()
            }
        };

        let mut assignments = Vec::new();
        for cw in course_work_list.iter().take(5) {
            let cw_id = match &cw.id {
                Some(id) => id.as_str(),
                None => continue,
            };

            let submissions = match self
                .hub
                .courses()
                .course_work_student_submissions_list(course_id, cw_id)
                .doit()
                .await
            {
                Ok((_resp, list)) => {
                    serde_json::to_value(list.student_submissions.unwrap_or_default())
                        .unwrap_or(json!([]))
                }
                Err(e) => {
                    tracing::warn!(
                        "failed to fetch submissions for {course_id}/{cw_id}: {e}"
                    );
                    json!([])
                }
            };

            assignments.push(json!({
                "courseWork": serde_json::to_value(cw).unwrap_or(json!(null)),
                "submissions": submissions,
            }));
        }

        // Include remaining coursework without submissions
        for cw in course_work_list.iter().skip(5) {
            assignments.push(json!({
                "courseWork": serde_json::to_value(cw).unwrap_or(json!(null)),
                "submissions": [],
            }));
        }

        Ok(json!({
            "course": serde_json::to_value(&course).map_err(AppError::Json)?,
            "assignments": assignments,
        }))
    }
}
