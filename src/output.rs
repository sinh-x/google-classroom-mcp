//! Output module for CLI tools: markdown generation and file writing.
//!
//! Writes both `.md` (with YAML frontmatter) and `.json` sidecar files.
//! Formatting logic lives in `formatters.rs`.

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
        let service_dir = self.output_dir.join(frontmatter.tool.split('/').next().unwrap_or("unknown"));
        std::fs::create_dir_all(&service_dir)?;

        let date_prefix = Utc::now().format("%Y-%m-%d").to_string();
        let md_name = format!("{}-{}.md", date_prefix, name);
        let json_name = format!("{}-{}.json", date_prefix, name);

        let md_path = service_dir.join(&md_name);
        let json_path = service_dir.join(&json_name);

        // Write JSON sidecar
        let json_content = serde_json::to_string_pretty(data)
            .map_err(AppError::Json)?;
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

        // Content (dispatched to formatters.rs)
        output.push_str(&self.format_data_as_markdown(&frontmatter.tool, data));

        output
    }

    /// Convert a snake_case or kebab-case name to Title Case.
    fn title_case(&self, name: &str) -> String {
        name.split(['_', '-'])
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
            } else if (c == '-' || c.is_whitespace()) && i > 0 && !slug.ends_with('-') {
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
