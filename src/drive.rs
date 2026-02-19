use std::time::Duration;

use http_body_util::BodyExt;
use moka::future::Cache;
use serde_json::{json, Value};

use crate::auth::DriveHubType;
use crate::error::AppError;

const MAX_CONTENT_BYTES: usize = 100 * 1024; // 100 KB
const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive.readonly";

pub struct DriveClient {
    hub: DriveHubType,
    memory_cache: Cache<String, Value>,
}

impl std::fmt::Debug for DriveClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DriveClient").finish_non_exhaustive()
    }
}

impl DriveClient {
    pub fn new(hub: DriveHubType) -> Self {
        let memory_cache = Cache::builder()
            .max_capacity(200)
            .time_to_live(Duration::from_secs(300))
            .build();

        Self { hub, memory_cache }
    }

    /// Read the content of a Google Drive file by file ID or URL.
    ///
    /// For Google Workspace documents (Docs, Sheets, Slides) the content is
    /// exported to a text format. For regular text files the content is
    /// downloaded directly. Binary files return metadata only.
    pub async fn read_material(&self, file_id_or_url: &str) -> Result<Value, AppError> {
        let file_id = parse_file_id(file_id_or_url)?;

        if let Some(cached) = self.memory_cache.get(&file_id).await {
            tracing::debug!("drive cache hit: {file_id}");
            return Ok(cached);
        }
        tracing::info!("drive cache miss, fetching metadata: {file_id}");

        // Fetch file metadata
        let (_resp, file) = self
            .hub
            .files()
            .get(&file_id)
            .param("fields", "id,name,mimeType,size,modifiedTime,webViewLink")
            .add_scope(DRIVE_SCOPE)
            .doit()
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("403") || msg.contains("insufficient") {
                    AppError::DriveApi(format!(
                        "Access denied for file {file_id}. You may need to re-authenticate \
                         with `cargo run -- auth` to grant the drive.readonly scope. \
                         Original error: {msg}"
                    ))
                } else {
                    AppError::DriveApi(msg)
                }
            })?;

        let mime_type = file.mime_type.as_deref().unwrap_or("unknown");
        let file_name = file.name.as_deref().unwrap_or("unknown");
        tracing::info!("file metadata: name={file_name}, mime={mime_type}");

        let metadata = json!({
            "id": file.id,
            "name": file.name,
            "mimeType": file.mime_type,
            "size": file.size,
            "modifiedTime": file.modified_time.map(|t| t.to_rfc3339()),
            "webViewLink": file.web_view_link,
        });

        // Determine how to fetch content based on mime type
        let (content, export_mime) = match mime_type {
            "application/vnd.google-apps.document" => {
                let text = self.export_file(&file_id, "text/plain").await?;
                (Some(text), Some("text/plain"))
            }
            "application/vnd.google-apps.spreadsheet" => {
                let csv = self.export_file(&file_id, "text/csv").await?;
                (Some(csv), Some("text/csv"))
            }
            "application/vnd.google-apps.presentation" => {
                let text = self.export_file(&file_id, "text/plain").await?;
                (Some(text), Some("text/plain"))
            }
            m if m.starts_with("text/")
                || m == "application/json"
                || m == "application/xml"
                || m == "application/javascript"
                || m == "application/x-yaml"
                || m == "application/csv" =>
            {
                let text = self.download_file(&file_id).await?;
                (Some(text), None)
            }
            _ => {
                // Binary / PDF / image — return metadata only
                (None, None)
            }
        };

        let (content_value, truncated) = match content {
            Some(text) => {
                let (truncated_text, was_truncated) = truncate_content(&text);
                (Value::String(truncated_text), was_truncated)
            }
            None => (Value::Null, false),
        };

        let result = json!({
            "metadata": metadata,
            "content": content_value,
            "exportedAs": export_mime,
            "truncated": truncated,
            "note": if content_value.is_null() {
                format!("Binary file ({mime_type}) — content not fetched. \
                         Name: {file_name}. Use the webViewLink to open in browser.")
            } else if truncated {
                format!("Content truncated to {MAX_CONTENT_BYTES} bytes.")
            } else {
                String::new()
            },
        });

        self.memory_cache.insert(file_id, result.clone()).await;
        Ok(result)
    }

    /// Export a Google Workspace document to the given MIME type.
    async fn export_file(&self, file_id: &str, mime_type: &str) -> Result<String, AppError> {
        tracing::info!("exporting {file_id} as {mime_type}");
        let resp = self
            .hub
            .files()
            .export(file_id, mime_type)
            .add_scope(DRIVE_SCOPE)
            .doit()
            .await
            .map_err(|e| AppError::DriveApi(format!("export failed for {file_id}: {e}")))?;

        let body = resp
            .into_body()
            .collect()
            .await
            .map_err(|e| AppError::DriveApi(format!("failed to read export body: {e}")))?
            .to_bytes();

        String::from_utf8(body.to_vec())
            .map_err(|e| AppError::DriveApi(format!("export produced invalid UTF-8: {e}")))
    }

    /// Download a regular (non-Workspace) file's content.
    async fn download_file(&self, file_id: &str) -> Result<String, AppError> {
        tracing::info!("downloading {file_id} via alt=media");
        let (resp, _file) = self
            .hub
            .files()
            .get(file_id)
            .param("alt", "media")
            .add_scope(DRIVE_SCOPE)
            .doit()
            .await
            .map_err(|e| AppError::DriveApi(format!("download failed for {file_id}: {e}")))?;

        let body = resp
            .into_body()
            .collect()
            .await
            .map_err(|e| AppError::DriveApi(format!("failed to read download body: {e}")))?
            .to_bytes();

        String::from_utf8(body.to_vec())
            .map_err(|e| AppError::DriveApi(format!("file is not valid UTF-8: {e}")))
    }
}

/// Extract a Google Drive file ID from a URL or return the input as-is if it
/// looks like a bare ID.
///
/// Supported URL patterns:
/// - `https://docs.google.com/document/d/{ID}/...`
/// - `https://drive.google.com/file/d/{ID}/...`
/// - `https://docs.google.com/spreadsheets/d/{ID}/...`
/// - `https://docs.google.com/presentation/d/{ID}/...`
/// - `https://drive.google.com/open?id={ID}`
fn parse_file_id(input: &str) -> Result<String, AppError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(AppError::InvalidInput(
            "file_id_or_url cannot be empty".into(),
        ));
    }

    // If it looks like a URL, try to extract the ID
    if input.starts_with("http://") || input.starts_with("https://") {
        // Pattern: /d/{ID}/
        if let Some(pos) = input.find("/d/") {
            let after = &input[pos + 3..];
            let id = after.split('/').next().unwrap_or("");
            if !id.is_empty() {
                return Ok(id.to_string());
            }
        }

        // Pattern: ?id={ID} or &id={ID}
        if let Some(pos) = input.find("id=") {
            let after = &input[pos + 3..];
            let id = after.split('&').next().unwrap_or("");
            if !id.is_empty() {
                return Ok(id.to_string());
            }
        }

        return Err(AppError::InvalidInput(format!(
            "could not extract file ID from URL: {input}"
        )));
    }

    // Bare file ID — basic validation (alphanumeric, hyphens, underscores)
    if input
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        Ok(input.to_string())
    } else {
        Err(AppError::InvalidInput(format!(
            "invalid file ID or URL: {input}"
        )))
    }
}

/// Truncate a string to at most `MAX_CONTENT_BYTES`, respecting UTF-8 char
/// boundaries. Returns `(text, was_truncated)`.
fn truncate_content(text: &str) -> (String, bool) {
    if text.len() <= MAX_CONTENT_BYTES {
        return (text.to_string(), false);
    }

    // Find the last valid char boundary at or before the limit
    let mut end = MAX_CONTENT_BYTES;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }

    let mut truncated = text[..end].to_string();
    truncated.push_str("\n\n[... content truncated at 100 KB ...]");
    (truncated, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_google_doc_url() {
        let url = "https://docs.google.com/document/d/1aBcDeFgHiJkLmNoPqRsTuVwXyZ/edit";
        assert_eq!(parse_file_id(url).unwrap(), "1aBcDeFgHiJkLmNoPqRsTuVwXyZ");
    }

    #[test]
    fn parse_drive_file_url() {
        let url = "https://drive.google.com/file/d/1aBcDeFgHiJkLmNoPqRsTuVwXyZ/view?usp=sharing";
        assert_eq!(parse_file_id(url).unwrap(), "1aBcDeFgHiJkLmNoPqRsTuVwXyZ");
    }

    #[test]
    fn parse_spreadsheet_url() {
        let url = "https://docs.google.com/spreadsheets/d/1aBcDeFgHiJkLmNoPqRsTuVwXyZ/edit#gid=0";
        assert_eq!(parse_file_id(url).unwrap(), "1aBcDeFgHiJkLmNoPqRsTuVwXyZ");
    }

    #[test]
    fn parse_open_id_url() {
        let url = "https://drive.google.com/open?id=1aBcDeFgHiJkLmNoPqRsTuVwXyZ";
        assert_eq!(parse_file_id(url).unwrap(), "1aBcDeFgHiJkLmNoPqRsTuVwXyZ");
    }

    #[test]
    fn parse_bare_file_id() {
        assert_eq!(
            parse_file_id("1aBcDeFgHiJkLmNoPqRsTuVwXyZ").unwrap(),
            "1aBcDeFgHiJkLmNoPqRsTuVwXyZ"
        );
    }

    #[test]
    fn parse_empty_input() {
        assert!(parse_file_id("").is_err());
        assert!(parse_file_id("  ").is_err());
    }

    #[test]
    fn truncate_short_text() {
        let (text, truncated) = truncate_content("hello");
        assert_eq!(text, "hello");
        assert!(!truncated);
    }

    #[test]
    fn truncate_long_text() {
        let long = "a".repeat(MAX_CONTENT_BYTES + 1000);
        let (text, truncated) = truncate_content(&long);
        assert!(truncated);
        assert!(text.len() <= MAX_CONTENT_BYTES + 50); // some room for the note
        assert!(text.ends_with("[... content truncated at 100 KB ...]"));
    }
}
