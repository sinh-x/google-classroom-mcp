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
- **`auth.rs`** — OAuth2 via `yup-oauth2` InstalledFlowAuthenticator. Config at `~/.config/personal-google-mcp/{credentials,tokens}.json`. Redirect on port 8085. Hub type aliases. `ProfileHubs` bundles all hubs for one profile. `discover_profiles()` scans config dir for authenticated profiles. `build_hubs_for_profile()` builds hubs for a single profile. `build_all_hubs()` discovers all profiles and builds hubs for each, returning `HashMap<String, ProfileHubs>`. `profile_dir_for()` returns the directory for a named profile. Auth still interactive via `PGM_PROFILE` env var — `profile_dir()` and `active_profile()` helpers.
- **`classroom.rs`** — `ClassroomClient` wrapping the `google-classroom1` hub with two-tier caching: `moka` in-memory (1000 entries, 5-min TTL) for all data, plus persistent JSON disk cache (never expires — survives restarts and loss of course access). Constructor takes explicit `cache_dir: PathBuf` (caller provides profile-specific path). Five async methods: `list_courses()`, `get_course_details()`, `get_assignments()`, `get_course_materials()`, `get_course_topics()`. Soft errors for sub-requests (announcements, submissions).
- **`drive.rs`** — `DriveClient` wrapping `google-drive3` hub with `moka` in-memory cache (200 entries, 5-min TTL). `read_material()` exports Google Workspace docs to text/CSV or downloads regular text files. Includes `parse_file_id()` for URL→ID extraction and 100 KB content truncation.
- **`tools.rs`** — `GoogleService` with `#[tool_router]` (10 tools) and `#[tool_handler]` for MCP. Holds `HashMap<String, ProfileClients>` for multi-account support. `ProfileClients` bundles `Arc<ClassroomClient>`, `Arc<DriveClient>`, `Arc<CalendarClient>`. All tools accept optional `profile` parameter. `resolve_profile()` helper defaults to the default profile. `list_profiles` tool returns available profiles.

### Adding a New Google Service

Follow this pattern (e.g., for Calendar):
1. Add scopes to `auth.rs::SCOPES`
2. Add dependency to `Cargo.toml` (e.g., `google-calendar3`)
3. Create `src/calendar.rs` with `CalendarClient` (mirror `classroom.rs` pattern)
4. Add hub type alias and field to `ProfileHubs` in `auth.rs`, build in `build_hubs_for_profile()`
5. Add `Arc<CalendarClient>` to `GoogleService` in `tools.rs`
6. Add `#[tool]` handlers for the new service
7. Re-auth to pick up new scopes: `cargo run -- auth`

### MCP Tools

All tools accept an optional `profile` parameter to select which Google account to use.

| Tool | Parameters | Description |
|------|-----------|-------------|
| `list_profiles` | — | List available profiles and which is default |
| `courses` | `profile?` | List all courses |
| `course_details` | `course_id`, `profile?` | Course + up to 20 announcements |
| `assignments` | `course_id`, `profile?` | Coursework + submissions for first 5 |
| `course_materials` | `course_id`, `profile?` | Posted resources (docs, links, videos) |
| `course_topics` | `course_id`, `profile?` | Topics (modules/sections) organizing content |
| `read_material` | `file_id_or_url`, `profile?` | Read Google Drive file content (Docs→text, Sheets→CSV) |
| `calendars` | `profile?` | List all calendars |
| `calendar_events` | `calendar_id`, `days_ahead?`, `profile?` | Upcoming events on a calendar |
| `calendar_event_details` | `calendar_id`, `event_id`, `profile?` | Full details for a specific event |

### Key Dependencies

- `rmcp` 0.15 — MCP server SDK (macros, stdio transport)
- `google-classroom1` 7.0 — Google Classroom API bindings
- `google-drive3` 7.0 — Google Drive API bindings (file read/export)
- `yup-oauth2` 12 — OAuth2 authentication
- `hyper-rustls` 0.27 — HTTPS connector
- `moka` 0.12 — In-memory async cache (5-min TTL, 1000-entry cap); materials/topics also persisted to disk

### Multi-Account Profiles

A single server instance discovers all authenticated profiles at startup and serves them all. Use the `profile` parameter on any tool to select which account to use.

**Authentication** (one profile at a time via `PGM_PROFILE` env var):

```bash
# Authenticate default profile
cargo run -- auth

# Authenticate a named profile
PGM_PROFILE=work cargo run -- auth
```

**File layout:**
- `~/.config/personal-google-mcp/credentials.json` — shared OAuth client config
- `~/.config/personal-google-mcp/tokens.json` — default profile tokens
- `~/.config/personal-google-mcp/{profile}/tokens.json` — named profile tokens
- `~/.config/personal-google-mcp/{profile}/cache/` — per-profile disk cache

**At startup**, the server scans the config directory for all directories containing `tokens.json`. Root-level = "default" profile, subdirectories = named profiles. Profiles that fail authentication are skipped with a warning.

#### Claude Desktop config example (single server, multiple accounts)

```json
{
  "mcpServers": {
    "personal-google": {
      "command": "personal-google-mcp",
      "args": ["run"]
    }
  }
}
```

Then use `list_profiles` to see available accounts and pass `"profile": "work"` to any tool.

## Environment

- Nix dev shell via `flake.nix` + `.envrc` (direnv)
- Helper commands in shell: `pgm-dev`, `pgm-build`, `pgm-auth`, `pgm-test`
- `RUST_LOG=info` default
