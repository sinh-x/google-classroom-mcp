# CLAUDE.md

## Common Commands

- `cargo build` — Build debug binary
- `cargo build --release` — Build release binary
- `cargo check` — Type-check without building
- `cargo clippy` — Lint (available in nix dev shell)
- `cargo run -- run` — Start MCP server on stdio
- `cargo run -- auth` — Run OAuth2 authentication flow

## Architecture

Rust MCP server bridging Google APIs with AI assistants via the Model Context Protocol. Designed as a unified server for personal Google services.

### Entry Point (`src/main.rs`)

Clap CLI with two subcommands:
- `run` (default) — builds API hubs from saved tokens, starts MCP server on stdio
- `auth` — runs interactive OAuth2 flow, saves tokens to disk

Tracing logs to stderr (stdout reserved for MCP stdio transport).

### Modules

- **`error.rs`** — `AppError` enum with `thiserror` derives
- **`auth.rs`** — OAuth2 via `yup-oauth2` InstalledFlowAuthenticator. Config at `~/.config/personal-google-mcp/{credentials,tokens}.json`. Redirect on port 8085. Hub type aliases. `build_hubs()` returns all hubs sharing one authenticator. Supports multi-profile via `PGM_PROFILE` env var — `profile_dir()` and `active_profile()` helpers.
- **`classroom.rs`** — `ClassroomClient` wrapping the `google-classroom1` hub with two-tier caching: `moka` in-memory (1000 entries, 5-min TTL) for all data, plus persistent JSON disk cache at `~/.config/personal-google-mcp/cache/` for materials and topics (never expires — survives restarts and loss of course access). Five async methods: `list_courses()`, `get_course_details()`, `get_assignments()`, `get_course_materials()`, `get_course_topics()`. Soft errors for sub-requests (announcements, submissions).
- **`drive.rs`** — `DriveClient` wrapping `google-drive3` hub with `moka` in-memory cache (200 entries, 5-min TTL). `read_material()` exports Google Workspace docs to text/CSV or downloads regular text files. Includes `parse_file_id()` for URL→ID extraction and 100 KB content truncation.
- **`tools.rs`** — `GoogleService` with `#[tool_router]` (6 tools) and `#[tool_handler]` for MCP. Uses `Arc<ClassroomClient>` and `Arc<DriveClient>` for Clone compatibility.

### Adding a New Google Service

Follow this pattern (e.g., for Calendar):
1. Add scopes to `auth.rs::SCOPES`
2. Add dependency to `Cargo.toml` (e.g., `google-calendar3`)
3. Create `src/calendar.rs` with `CalendarClient` (mirror `classroom.rs` pattern)
4. Add hub type alias and build in `auth.rs::build_hubs()`
5. Add `Arc<CalendarClient>` to `GoogleService` in `tools.rs`
6. Add `#[tool]` handlers for the new service
7. Re-auth to pick up new scopes: `cargo run -- auth`

### MCP Tools

| Tool | Parameters | Description |
|------|-----------|-------------|
| `courses` | none | List all courses |
| `course_details` | `course_id` | Course + up to 20 announcements |
| `assignments` | `course_id` | Coursework + submissions for first 5 |
| `course_materials` | `course_id` | Posted resources (docs, links, videos) |
| `course_topics` | `course_id` | Topics (modules/sections) organizing content |
| `read_material` | `file_id_or_url` | Read Google Drive file content (Docs→text, Sheets→CSV) |

### Key Dependencies

- `rmcp` 0.15 — MCP server SDK (macros, stdio transport)
- `google-classroom1` 7.0 — Google Classroom API bindings
- `google-drive3` 7.0 — Google Drive API bindings (file read/export)
- `yup-oauth2` 12 — OAuth2 authentication
- `hyper-rustls` 0.27 — HTTPS connector
- `moka` 0.12 — In-memory async cache (5-min TTL, 1000-entry cap); materials/topics also persisted to disk

### Multi-Account Profiles

Set `PGM_PROFILE` env var to use a named profile for a different Google account:

```bash
# Authenticate a new profile
PGM_PROFILE=work cargo run -- auth

# Run server with that profile
PGM_PROFILE=work cargo run -- run
```

File layout:
- `~/.config/personal-google-mcp/credentials.json` — shared OAuth client config
- `~/.config/personal-google-mcp/{profile}/tokens.json` — per-profile tokens
- `~/.config/personal-google-mcp/{profile}/cache/` — per-profile disk cache

When `PGM_PROFILE` is unset, the server uses the default (root-level) config for backward compatibility.

#### Claude Desktop config example (multiple accounts)

```json
{
  "mcpServers": {
    "google-personal": {
      "command": "personal-google-mcp",
      "args": ["run"]
    },
    "google-work": {
      "command": "personal-google-mcp",
      "args": ["run"],
      "env": {
        "PGM_PROFILE": "work"
      }
    }
  }
}
```

## Environment

- Nix dev shell via `flake.nix` + `.envrc` (direnv)
- Helper commands in shell: `pgm-dev`, `pgm-build`, `pgm-auth`, `pgm-test`
- `RUST_LOG=info` default
