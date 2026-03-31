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
            if let Some(obj) = params.as_object() {
                if !obj.is_empty() {
                    output.push_str("params:\n");
                    for (k, v) in obj {
                        output.push_str(&format!("  {}: {}\n", k, v));
                    }
                }
            }
        }
        output.push_str("---\n\n");

        // Title
        output.push_str("# ");
        output.push_str(&self.title_case(name));
        output.push_str("\n\n");

        // Content
        output.push_str(&self.format_data_as_markdown(&frontmatter.tool, data));

        output
    }

    /// Format JSON data as human-readable markdown.
    /// Extracts the formatter suffix from frontmatter.tool (e.g., "classroom/courses" -> "courses")
    /// to route to the correct specialized formatter.
    fn format_data_as_markdown(&self, tool: &str, data: &Value) -> String {
        let suffix = tool.split('/').last().unwrap_or(tool);
        match suffix {
            "courses" => self.format_courses(data),
            "details" => self.format_course_details(data),
            "assignments" => self.format_assignments(data),
            "materials" => self.format_materials(data),
            "topics" => self.format_topics(data),
            "calendars" => self.format_calendars(data),
            "events" => self.format_events(data),
            "event-details" => self.format_event_details(data),
            "read" => self.format_drive_read(data),
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
        if let Some(obj) = data.as_object() {
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
    /// Strips non-alphanumeric characters except hyphens, converts to lowercase, replaces spaces with hyphens.
    pub fn slugify(name: &str) -> String {
        let mut slug = String::new();
        for (i, c) in name.chars().enumerate() {
            if c.is_alphanumeric() {
                slug.push(c.to_ascii_lowercase());
            } else if c == '-' && i > 0 && !slug.ends_with('-') {
                slug.push('-');
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
    fn test_slugify_edge_cases() {
        // Special characters - # is stripped (not alphanumeric, hyphen, or space)
        assert_eq!(OutputWriter::slugify("Hello! World?"), "hello-world");
        assert_eq!(OutputWriter::slugify("file#1.txt"), "file1txt");
        // Unicode - trailing hyphen trimmed, multiple spaces collapse to one hyphen
        assert_eq!(OutputWriter::slugify("日本語"), "日本語");
        assert_eq!(OutputWriter::slugify("émojis 🎉"), "émojis");
        // Empty string
        assert_eq!(OutputWriter::slugify(""), "");
        // Leading/trailing hyphens
        assert_eq!(OutputWriter::slugify("-leading-hyphen-"), "leading-hyphen");
        // Multiple spaces - spaces after first are skipped because slug already ends with '-'
        assert_eq!(OutputWriter::slugify("multiple   spaces"), "multiple-spaces");
    }

    #[test]
    fn test_title_case() {
        let writer = OutputWriter::new("default".to_string(), PathBuf::from("/tmp"));
        assert_eq!(writer.title_case("courses"), "Courses");
        assert_eq!(writer.title_case("course_details"), "Course Details");
        assert_eq!(writer.title_case("drive-read"), "Drive Read");
    }

    #[test]
    fn test_format_courses_with_sample_json() {
        let writer = OutputWriter::new("default".to_string(), PathBuf::from("/tmp"));
        let sample_courses = serde_json::json!([
            {
                "id": "123",
                "name": "Introduction to Computer Science",
                "courseState": "ACTIVE",
                "section": "A"
            },
            {
                "id": "456",
                "name": "Advanced Programming",
                "courseState": "ARCHIVED",
                "section": "B"
            }
        ]);
        let result = writer.format_courses(&sample_courses);
        assert!(result.contains("| Introduction to Computer Science | 123 | ACTIVE | A |"));
        assert!(result.contains("| Advanced Programming | 456 | ARCHIVED | B |"));
    }

    #[test]
    fn test_format_events_with_sample_json() {
        let writer = OutputWriter::new("default".to_string(), PathBuf::from("/tmp"));
        let sample_events = serde_json::json!([
            {
                "summary": "Team Meeting",
                "start": { "dateTime": "2026-03-31T10:00:00+07:00" },
                "end": { "dateTime": "2026-03-31T11:00:00+07:00" },
                "location": "Conference Room A"
            },
            {
                "summary": "Lunch Break",
                "start": { "date": "2026-03-31" },
                "end": { "date": "2026-03-31" },
                "location": ""
            }
        ]);
        let result = writer.format_events(&sample_events);
        assert!(result.contains("| Team Meeting |"));
        assert!(result.contains("| 2026-03-31T10:00:00+07:00 |"));
        assert!(result.contains("| Conference Room A |"));
        assert!(result.contains("| Lunch Break |"));
    }

    #[test]
    fn test_format_drive_read_with_sample_json() {
        let writer = OutputWriter::new("default".to_string(), PathBuf::from("/tmp"));
        let sample_drive = serde_json::json!({
            "metadata": {
                "id": "abc123",
                "name": "Test Document",
                "mimeType": "application/vnd.google-apps.document"
            },
            "content": "This is the document content.",
            "note": "Important document"
        });
        let result = writer.format_drive_read(&sample_drive);
        assert!(result.contains("## File Metadata"));
        assert!(result.contains("| Field | Value |"));
        assert!(result.contains("| id | abc123 |"));
        assert!(result.contains("## Content"));
        assert!(result.contains("This is the document content."));
        assert!(result.contains("> Important document"));
    }

    #[test]
    fn test_format_data_as_markdown_routing() {
        let writer = OutputWriter::new("default".to_string(), PathBuf::from("/tmp"));
        let sample_courses = serde_json::json!([
            {"id": "1", "name": "Test", "courseState": "ACTIVE", "section": "A"}
        ]);
        let sample_events = serde_json::json!([
            {"summary": "Event", "start": {"dateTime": "2026-03-31T10:00:00+07:00"}, "end": {"dateTime": "2026-03-31T11:00:00+07:00"}}
        ]);
        let sample_drive = serde_json::json!({"metadata": {"id": "1", "name": "Doc"}});

        // Test courses routing
        let courses_output = writer.format_data_as_markdown("classroom/courses", &sample_courses);
        assert!(courses_output.contains("| Name | ID | State | Section |"));

        // Test events routing
        let events_output = writer.format_data_as_markdown("calendar/events", &sample_events);
        assert!(events_output.contains("| Summary | Start | End | Location |"));

        // Test drive read routing
        let drive_output = writer.format_data_as_markdown("drive/read", &sample_drive);
        assert!(drive_output.contains("## File Metadata"));

        // Test unknown tool falls back to generic
        let generic_output = writer.format_data_as_markdown("unknown/tool", &serde_json::json!({}));
        // Generic should output JSON or key-value table, not course/event format
        assert!(!generic_output.contains("| Name | ID | State | Section |"));
        assert!(!generic_output.contains("| Summary | Start | End | Location |"));
    }

    #[test]
    fn test_write_output_creates_md_and_json_files() {
        let temp_dir = std::env::temp_dir().join("output_writer_test");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let writer = OutputWriter::new("test".to_string(), temp_dir.clone());

        let data = serde_json::json!({"name": "Test Course", "id": "123"});
        let frontmatter = Frontmatter {
            tool: "classroom/courses".to_string(),
            profile: "test".to_string(),
            date: "2026-03-31T00:00:00+00:00".to_string(),
            params: None,
        };

        let result = writer.write_output("test-course", &data, &frontmatter);
        assert!(result.is_ok());

        let md_path = result.unwrap();
        assert!(md_path.exists());
        assert!(md_path.extension().unwrap() == "md");

        // Check JSON sidecar was created
        let json_path = md_path.with_extension("json");
        assert!(json_path.exists());

        // Verify JSON content
        let json_content = std::fs::read_to_string(&json_path).unwrap();
        assert!(json_content.contains("Test Course"));
        assert!(json_content.contains("123"));

        // Verify markdown content
        let md_content = std::fs::read_to_string(&md_path).unwrap();
        assert!(md_content.contains("---"));
        assert!(md_content.contains("tool: classroom/courses"));
        assert!(md_content.contains("# Test Course"));

        // Cleanup
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}