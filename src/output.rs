//! Output module for CLI tools: markdown generation and file writing.
//!
//! Writes both `.md` (with YAML frontmatter) and `.json` sidecar files.

use chrono::Utc;
use serde_json::Value;
use std::path::PathBuf;

use crate::error::AppError;

/// Frontmatter metadata for output files.
#[derive(Debug, Clone)]
pub struct Frontmatter {
    /// The tool name, e.g. "classroom/courses"
    pub tool: String,
    /// The profile name used
    pub profile: String,
    /// ISO 8601 timestamp
    pub date: String,
    /// Optional additional parameters
    pub params: Option<Value>,
}

/// Output writer for a single profile's output directory.
#[derive(Debug)]
pub struct OutputWriter {
    /// Profile name (e.g. "default", "work")
    #[allow(dead_code)]
    profile: String,
    /// Base output directory (e.g. ~/.local/share/personal-google-mcp/{profile}/)
    output_dir: PathBuf,
}

impl OutputWriter {
    /// Create a new OutputWriter.
    pub fn new(profile: String, output_dir: PathBuf) -> Self {
        Self { profile, output_dir }
    }

    /// Write data to both markdown and JSON files.
    ///
    /// Returns the path to the written markdown file.
    pub fn write_output(
        &self,
        name: &str,
        data: &Value,
        frontmatter: &Frontmatter,
    ) -> Result<PathBuf, AppError> {
        let service_dir = self.output_dir.join(&frontmatter.tool.split('/').next().unwrap_or("unknown"));
        std::fs::create_dir_all(&service_dir)?;

        let date_prefix = Utc::now().format("%Y-%m-%d").to_string();
        let md_name = format!("{}-{}.md", date_prefix, name);
        let json_name = format!("{}-{}.json", date_prefix, name);

        let md_path = service_dir.join(&md_name);
        let json_path = service_dir.join(&json_name);

        // Write JSON sidecar
        let json_content = serde_json::to_string_pretty(data)
            .map_err(|e| AppError::Json(e))?;
        std::fs::write(&json_path, json_content)?;

        // Write markdown with frontmatter
        let markdown = self.format_markdown(name, data, frontmatter);
        std::fs::write(&md_path, markdown)?;

        Ok(md_path)
    }

    /// Format data as markdown with YAML frontmatter.
    fn format_markdown(
        &self,
        name: &str,
        data: &Value,
        frontmatter: &Frontmatter,
    ) -> String {
        let mut output = String::new();

        // YAML frontmatter
        output.push_str("---\n");
        output.push_str(&format!("tool: {}\n", frontmatter.tool));
        output.push_str(&format!("profile: {}\n", frontmatter.profile));
        output.push_str(&format!("date: {}\n", frontmatter.date));
        if let Some(params) = &frontmatter.params {
            if params.is_object() && !params.as_object().unwrap().is_empty() {
                output.push_str("params:\n");
                for (k, v) in params.as_object().unwrap() {
                    output.push_str(&format!("  {}: {}\n", k, v));
                }
            }
        }
        output.push_str("---\n\n");

        // Title
        output.push_str("# ");
        output.push_str(&self.title_case(name));
        output.push_str("\n\n");

        // Content
        output.push_str(&self.format_data_as_markdown(name, data));

        output
    }

    /// Format JSON data as human-readable markdown.
    fn format_data_as_markdown(&self, name: &str, data: &Value) -> String {
        match name {
            "courses" => self.format_courses(data),
            "details" => self.format_course_details(data),
            "assignments" => self.format_assignments(data),
            "materials" => self.format_materials(data),
            "topics" => self.format_topics(data),
            "calendars" => self.format_calendars(data),
            "events" => self.format_events(data),
            "event-details" => self.format_event_details(data),
            "drive-read" => self.format_drive_read(data),
            _ => self.format_generic(data),
        }
    }

    fn format_courses(&self, data: &Value) -> String {
        let courses: &[Value] = data.as_array().map(|v| v.as_slice()).unwrap_or(&[]);
        if courses.is_empty() {
            return "No courses found.\n".to_string();
        }

        let mut output = String::new();
        output.push_str("| Name | ID | State | Section |\n");
        output.push_str("|------|----|-------|--------|\n");

        for course in courses {
            let name = course.get("name").and_then(|v| v.as_str()).unwrap_or("—");
            let id = course.get("id").and_then(|v| v.as_str()).unwrap_or("—");
            let state = course.get("courseState").and_then(|v| v.as_str()).unwrap_or("—");
            let section = course.get("section").and_then(|v| v.as_str()).unwrap_or("—");
            output.push_str(&format!("| {} | {} | {} | {} |\n", name, id, state, section));
        }
        output
    }

    fn format_course_details(&self, data: &Value) -> String {
        let mut output = String::new();

        if let Some(course) = data.get("course") {
            output.push_str("## Course\n\n");
            output.push_str(&self.format_generic(course));
            output.push_str("\n\n");
        }

        if let Some(announcements) = data.get("announcements") {
            let list: &[Value] = announcements.as_array().map(|v| v.as_slice()).unwrap_or(&[]);
            output.push_str("## Recent Announcements\n\n");
            if list.is_empty() {
                output.push_str("No announcements.\n");
            } else {
                for ann in list.iter().take(10) {
                    let text = ann.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let updated = ann.get("updateTime").and_then(|v| v.as_str()).unwrap_or("");
                    output.push_str(&format!("- **{}** (updated: {})\n  \n  {}\n",
                        ann.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled"),
                        updated,
                        text.chars().take(200).collect::<String>()
                    ));
                }
            }
        }

        output
    }

    fn format_assignments(&self, data: &Value) -> String {
        let mut output = String::new();

        if let Some(course) = data.get("course") {
            let name = course.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown Course");
            output.push_str(&format!("## Course: {}\n\n", name));
        }

        let assignments: &[Value] = data.get("assignments").and_then(|v| v.as_array()).map(|v| v.as_slice()).unwrap_or(&[]);
        if assignments.is_empty() {
            return "No assignments found.\n".to_string();
        }

        output.push_str("| Title | Due Date | Type | State |\n");
        output.push_str("|-------|----------|------|-------|\n");

        for item in assignments {
            let cw = item.get("courseWork");
            let title = cw.and_then(|v| v.get("title").and_then(|v| v.as_str())).unwrap_or("—");
            let due = cw.and_then(|v| v.get("dueDate")).map(|v| v.to_string()).unwrap_or_else(|| "—".to_string());
            let work_type = cw.and_then(|v| v.get("workType").and_then(|v| v.as_str())).unwrap_or("—");
            let state = cw.and_then(|v| v.get("state").and_then(|v| v.as_str())).unwrap_or("—");
            output.push_str(&format!("| {} | {} | {} | {} |\n", title, due, work_type, state));
        }

        output
    }

    fn format_materials(&self, data: &Value) -> String {
        let materials: &[Value] = data.as_array().map(|v| v.as_slice()).unwrap_or(&[]);
        if materials.is_empty() {
            return "No materials found.\n".to_string();
        }

        let mut output = String::new();
        output.push_str("| Title | Type | State |\n");
        output.push_str("|-------|------|-------|\n");

        for mat in materials {
            let title = mat.get("title").and_then(|v| v.as_str()).unwrap_or("—");
            let mat_type = mat.get("materialType").and_then(|v| v.as_str()).unwrap_or("—");
            let state = mat.get("state").and_then(|v| v.as_str()).unwrap_or("—");
            output.push_str(&format!("| {} | {} | {} |\n", title, mat_type, state));
        }
        output
    }

    fn format_topics(&self, data: &Value) -> String {
        let topics: &[Value] = data.as_array().map(|v| v.as_slice()).unwrap_or(&[]);
        if topics.is_empty() {
            return "No topics found.\n".to_string();
        }

        let mut output = String::new();
        output.push_str("| Topic | ID |\n");
        output.push_str("|-------|----|\n");

        for topic in topics {
            let name = topic.get("name").and_then(|v| v.as_str()).unwrap_or("—");
            let id = topic.get("topicId").and_then(|v| v.as_str()).unwrap_or("—");
            output.push_str(&format!("| {} | {} |\n", name, id));
        }
        output
    }

    fn format_calendars(&self, data: &Value) -> String {
        let calendars: &[Value] = data.as_array().map(|v| v.as_slice()).unwrap_or(&[]);
        if calendars.is_empty() {
            return "No calendars found.\n".to_string();
        }

        let mut output = String::new();
        output.push_str("| Summary | ID | Role | Timezone |\n");
        output.push_str("|---------|----|------|----------|\n");

        for cal in calendars {
            let summary = cal.get("summary").and_then(|v| v.as_str()).unwrap_or("—");
            let id = cal.get("id").and_then(|v| v.as_str()).unwrap_or("—");
            let role = cal.get("accessRole").and_then(|v| v.as_str()).unwrap_or("—");
            let tz = cal.get("timeZone").and_then(|v| v.as_str()).unwrap_or("—");
            let primary = cal.get("primary").and_then(|v| v.as_bool()).unwrap_or(false);
            let summary_display = if primary { format!("{} ★", summary) } else { summary.to_string() };
            output.push_str(&format!("| {} | {} | {} | {} |\n", summary_display, id, role, tz));
        }
        output
    }

    fn format_events(&self, data: &Value) -> String {
        let events: &[Value] = data.as_array().map(|v| v.as_slice()).unwrap_or(&[]);
        if events.is_empty() {
            return "No upcoming events.\n".to_string();
        }

        let mut output = String::new();
        output.push_str("| Summary | Start | End | Location |\n");
        output.push_str("|---------|-------|-----|----------|\n");

        for evt in events {
            let summary = evt.get("summary").and_then(|v| v.as_str()).unwrap_or("—");
            let start = evt.get("start").and_then(|v| v.get("dateTime").or(v.get("date"))).and_then(|v| v.as_str()).unwrap_or("—");
            let end = evt.get("end").and_then(|v| v.get("dateTime").or(v.get("date"))).and_then(|v| v.as_str()).unwrap_or("—");
            let location = evt.get("location").and_then(|v| v.as_str()).unwrap_or("—");
            output.push_str(&format!("| {} | {} | {} | {} |\n", summary, start, end, location));
        }
        output
    }

    fn format_event_details(&self, data: &Value) -> String {
        self.format_generic(data)
    }

    fn format_drive_read(&self, data: &Value) -> String {
        let mut output = String::new();

        if let Some(metadata) = data.get("metadata") {
            output.push_str("## File Metadata\n\n");
            output.push_str(&self.format_generic(metadata));
            output.push_str("\n\n");
        }

        if let Some(content) = data.get("content") {
            if content.is_string() {
                output.push_str("## Content\n\n");
                output.push_str("```\n");
                output.push_str(content.as_str().unwrap_or(""));
                output.push_str("\n```\n");
            }
        }

        if let Some(note) = data.get("note").and_then(|v| v.as_str()) {
            if !note.is_empty() {
                output.push_str(&format!("\n> {}\n", note));
            }
        }

        if output.is_empty() {
            output.push_str(&self.format_generic(data));
        }

        output
    }

    /// Generic JSON-to-markdown formatter for tables of key-value pairs or arrays.
    fn format_generic(&self, data: &Value) -> String {
        if data.is_object() {
            let obj = data.as_object().unwrap();
            if obj.values().all(|v| !v.is_array()) {
                // Key-value table
                let mut output = String::new();
                output.push_str("| Field | Value |\n");
                output.push_str("|-------|-------|\n");
                for (k, v) in obj {
                    let value_str = match v {
                        Value::String(s) => s.clone(),
                        Value::Null => "—".to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Number(n) => n.to_string(),
                        Value::Array(arr) => {
                            if arr.is_empty() {
                                "[]".to_string()
                            } else {
                                serde_json::to_string_pretty(arr).unwrap_or_else(|_| v.to_string())
                            }
                        }
                        Value::Object(_inner_obj) => {
                            serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
                        }
                    };
                    output.push_str(&format!("| {} | {} |\n", k, value_str));
                }
                output
            } else {
                // Fall back to code block for mixed/complex data
                serde_json::to_string_pretty(data).unwrap_or_else(|_| data.to_string())
            }
        } else if data.is_array() {
            serde_json::to_string_pretty(data).unwrap_or_else(|_| data.to_string())
        } else {
            data.to_string()
        }
    }

    /// Convert a snake_case or kebab-case name to Title Case.
    fn title_case(&self, name: &str) -> String {
        name.split(|c: char| c == '_' || c == '-')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Slugify a string for use in filenames.
    /// Strips non-alphanumeric characters, converts to lowercase, replaces spaces with hyphens.
    pub fn slugify(name: &str) -> String {
        let mut slug = String::new();
        for (i, c) in name.chars().enumerate() {
            if c.is_alphanumeric() {
                slug.push(c.to_ascii_lowercase());
            } else if c.is_whitespace() && i > 0 && !slug.ends_with('-') {
                slug.push('-');
            }
        }
        slug.trim_matches('-').to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(OutputWriter::slugify("Math 101"), "math-101");
        assert_eq!(OutputWriter::slugify("Introduction to Computer Science"), "introduction-to-computer-science");
        assert_eq!(OutputWriter::slugify("Test@#$File"), "testfile");
        assert_eq!(OutputWriter::slugify("already-slugified"), "already-slugified");
    }

    #[test]
    fn test_title_case() {
        let writer = OutputWriter::new("default".to_string(), PathBuf::from("/tmp"));
        assert_eq!(writer.title_case("courses"), "Courses");
        assert_eq!(writer.title_case("course_details"), "Course Details");
        assert_eq!(writer.title_case("drive-read"), "Drive Read");
    }
}