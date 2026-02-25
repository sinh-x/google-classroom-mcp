use std::path::PathBuf;
use std::time::Duration;

use moka::future::Cache;
use serde_json::{json, Value};

use crate::auth::ClassroomHub;
use crate::error::AppError;

pub struct ClassroomClient {
    hub: ClassroomHub,
    memory_cache: Cache<String, Value>,
    cache_dir: PathBuf,
}

impl std::fmt::Debug for ClassroomClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClassroomClient").finish_non_exhaustive()
    }
}

impl ClassroomClient {
    pub fn new(hub: ClassroomHub) -> Self {
        let memory_cache = Cache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(300))
            .build();

        let cache_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("personal-google-mcp")
            .join("cache");

        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            tracing::warn!("failed to create disk cache directory: {e}");
        }

        Self {
            hub,
            memory_cache,
            cache_dir,
        }
    }

    /// Read a value from the disk cache.
    fn read_disk_cache(&self, key: &str) -> Option<Value> {
        let path = self.cache_dir.join(format!("{key}.json"));
        match std::fs::read_to_string(&path) {
            Ok(data) => match serde_json::from_str(&data) {
                Ok(val) => {
                    tracing::debug!("disk cache hit: {key}");
                    Some(val)
                }
                Err(e) => {
                    tracing::warn!("disk cache corrupted for {key}: {e}");
                    None
                }
            },
            Err(_) => None,
        }
    }

    /// Write a value to the disk cache.
    fn write_disk_cache(&self, key: &str, value: &Value) {
        let path = self.cache_dir.join(format!("{key}.json"));
        match serde_json::to_string_pretty(value) {
            Ok(data) => {
                if let Err(e) = std::fs::write(&path, data) {
                    tracing::warn!("failed to write disk cache for {key}: {e}");
                }
            }
            Err(e) => {
                tracing::warn!("failed to serialize for disk cache {key}: {e}");
            }
        }
    }

    /// List all courses the authenticated user can see.
    pub async fn list_courses(&self) -> Result<Value, AppError> {
        let key = "courses".to_string();
        if let Some(cached) = self.memory_cache.get(&key).await {
            tracing::debug!("memory cache hit: {key}");
            return Ok(cached);
        }
        tracing::debug!("memory cache miss: {key}");

        let (_resp, list) = self
            .hub
            .courses()
            .list()
            .page_size(100)
            .doit()
            .await
            .map_err(|e| AppError::GoogleApi(e.to_string()))?;

        let courses = list.courses.unwrap_or_default();
        let value = serde_json::to_value(&courses).map_err(AppError::Json)?;
        self.memory_cache.insert(key, value.clone()).await;
        Ok(value)
    }

    /// Get course details plus its most recent announcements.
    pub async fn get_course_details(&self, course_id: &str) -> Result<Value, AppError> {
        let key = format!("course_details:{course_id}");
        if let Some(cached) = self.memory_cache.get(&key).await {
            tracing::debug!("memory cache hit: {key}");
            return Ok(cached);
        }
        tracing::debug!("memory cache miss: {key}");

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

        let value = json!({
            "course": serde_json::to_value(&course).map_err(AppError::Json)?,
            "announcements": announcements,
        });
        self.memory_cache.insert(key, value.clone()).await;
        Ok(value)
    }

    /// Get coursework for a course plus student submissions for the first 5 assignments.
    pub async fn get_assignments(&self, course_id: &str) -> Result<Value, AppError> {
        let key = format!("assignments:{course_id}");
        if let Some(cached) = self.memory_cache.get(&key).await {
            tracing::debug!("memory cache hit: {key}");
            return Ok(cached);
        }
        tracing::debug!("memory cache miss: {key}");

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

        let value = json!({
            "course": serde_json::to_value(&course).map_err(AppError::Json)?,
            "assignments": assignments,
        });
        self.memory_cache.insert(key, value.clone()).await;
        Ok(value)
    }

    /// Get course work materials (posted resources) for a course.
    /// Results are persisted to disk so they survive restarts and remain
    /// available even after losing access to the course.
    pub async fn get_course_materials(&self, course_id: &str) -> Result<Value, AppError> {
        let key = format!("materials_{course_id}");

        // 1. Memory cache
        if let Some(cached) = self.memory_cache.get(&key).await {
            tracing::debug!("memory cache hit: {key}");
            return Ok(cached);
        }

        // 2. Disk cache (persistent)
        if let Some(cached) = self.read_disk_cache(&key) {
            self.memory_cache.insert(key, cached.clone()).await;
            return Ok(cached);
        }

        tracing::debug!("cache miss (memory + disk): {key}");

        // 3. Fetch from API
        let materials = match self
            .hub
            .courses()
            .course_work_materials_list(course_id)
            .page_size(50)
            .doit()
            .await
        {
            Ok((_resp, list)) => list.course_work_material.unwrap_or_default(),
            Err(e) => {
                return Err(AppError::GoogleApi(format!(
                    "failed to fetch course materials for {course_id}: {e}"
                )));
            }
        };

        let value = serde_json::to_value(&materials).map_err(AppError::Json)?;

        // Save to both caches
        self.memory_cache.insert(key.clone(), value.clone()).await;
        self.write_disk_cache(&key, &value);

        Ok(value)
    }

    /// Get topics (modules/sections) for a course.
    /// Results are persisted to disk so they survive restarts and remain
    /// available even after losing access to the course.
    pub async fn get_course_topics(&self, course_id: &str) -> Result<Value, AppError> {
        let key = format!("topics_{course_id}");

        // 1. Memory cache
        if let Some(cached) = self.memory_cache.get(&key).await {
            tracing::debug!("memory cache hit: {key}");
            return Ok(cached);
        }

        // 2. Disk cache (persistent)
        if let Some(cached) = self.read_disk_cache(&key) {
            self.memory_cache.insert(key, cached.clone()).await;
            return Ok(cached);
        }

        tracing::debug!("cache miss (memory + disk): {key}");

        // 3. Fetch from API
        let topics = match self
            .hub
            .courses()
            .topics_list(course_id)
            .page_size(100)
            .doit()
            .await
        {
            Ok((_resp, list)) => list.topic.unwrap_or_default(),
            Err(e) => {
                return Err(AppError::GoogleApi(format!(
                    "failed to fetch topics for {course_id}: {e}"
                )));
            }
        };

        let value = serde_json::to_value(&topics).map_err(AppError::Json)?;

        // Save to both caches
        self.memory_cache.insert(key.clone(), value.clone()).await;
        self.write_disk_cache(&key, &value);

        Ok(value)
    }
}
