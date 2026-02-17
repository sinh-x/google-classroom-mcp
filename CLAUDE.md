# CLAUDE.md

## Common Commands

- `cargo build` — Build debug binary
- `cargo build --release` — Build release binary
- `cargo check` — Type-check without building
- `cargo clippy` — Lint (available in nix dev shell)
- `cargo run -- run` — Start MCP server on stdio
- `cargo run -- auth` — Run OAuth2 authentication flow

## Architecture

Rust MCP server bridging Google Classroom API with AI assistants via the Model Context Protocol.

### Entry Point (`src/main.rs`)

Clap CLI with two subcommands:
- `run` (default) — builds Classroom hub from saved tokens, starts MCP server on stdio
- `auth` — runs interactive OAuth2 flow, saves tokens to disk

Tracing logs to stderr (stdout reserved for MCP stdio transport).

### Modules

- **`error.rs`** — `AppError` enum with `thiserror` derives
- **`auth.rs`** — OAuth2 via `yup-oauth2` InstalledFlowAuthenticator. Config at `~/.config/google-classroom-mcp/{credentials,tokens}.json`. Redirect on port 8085. `ClassroomHub` type alias for the verbose generic.
- **`classroom.rs`** — `ClassroomClient` wrapping the `google-classroom1` hub. Three async methods: `list_courses()`, `get_course_details()`, `get_assignments()`. Soft errors for sub-requests (announcements, submissions).
- **`tools.rs`** — `ClassroomService` with `#[tool_router]` (3 tools) and `#[tool_handler]` for MCP. Uses `Arc<ClassroomClient>` for Clone compatibility.

### MCP Tools

| Tool | Parameters | Description |
|------|-----------|-------------|
| `courses` | none | List all courses |
| `course_details` | `course_id` | Course + up to 20 announcements |
| `assignments` | `course_id` | Coursework + submissions for first 5 |

### Key Dependencies

- `rmcp` 0.15 — MCP server SDK (macros, stdio transport)
- `google-classroom1` 7.0 — Google Classroom API bindings
- `yup-oauth2` 12 — OAuth2 authentication
- `hyper-rustls` 0.27 — HTTPS connector

## Environment

- Nix dev shell via `flake.nix` + `.envrc` (direnv)
- Helper commands in shell: `gcm-dev`, `gcm-build`, `gcm-auth`, `gcm-test`
- `RUST_LOG=info` default
