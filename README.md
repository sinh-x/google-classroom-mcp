# Personal Google MCP Server

A Rust [MCP](https://modelcontextprotocol.io/) server that provides access to personal Google services — Classroom, Calendar, and Drive — via both MCP tools and CLI subcommands.

## MCP Tools

All tools accept an optional `profile` parameter to select which Google account to use.

| Tool | Parameters | Description |
|------|-----------|-------------|
| `list_profiles` | — | List available profiles and which is default |
| `courses` | `profile?` | List all courses |
| `course_details` | `course_id`, `profile?` | Course info + up to 20 announcements |
| `assignments` | `course_id`, `profile?` | Coursework + submissions for first 5 |
| `course_materials` | `course_id`, `profile?` | Posted resources (docs, links, videos) |
| `course_topics` | `course_id`, `profile?` | Topics (modules/sections) organizing content |
| `read_material` | `file_id_or_url`, `profile?` | Read Google Drive file content (Docs→text, Sheets→CSV) |
| `calendars` | `profile?` | List all calendars |
| `calendar_events` | `calendar_id`, `days_ahead?`, `profile?` | Upcoming events on a calendar |
| `calendar_event_details` | `calendar_id`, `event_id`, `profile?` | Full details for a specific event |

## CLI Tools

CLI subcommands mirror MCP tools for terminal/script usage. All commands save output as local files.

### Global Flags

| Flag | Default | Description |
|------|---------|-------------|
| `--profile <name>` | `default` | Google account profile to use |
| `--output-dir <path>` | `~/.local/share/personal-google-mcp/{profile}/` | Override output directory |

### Classroom

```sh
personal-google-mcp classroom courses [--profile P]
personal-google-mcp classroom details <course_id> [--profile P]
personal-google-mcp classroom assignments <course_id> [--profile P]
personal-google-mcp classroom materials <course_id> [--profile P]
personal-google-mcp classroom topics <course_id> [--profile P]
```

### Calendar

```sh
personal-google-mcp calendar list [--profile P]
personal-google-mcp calendar events <calendar_id> [--days-ahead N] [--profile P]
personal-google-mcp calendar details <calendar_id> <event_id> [--profile P]
```

### Drive

```sh
personal-google-mcp drive read <file_id_or_url> [--profile P]
```

Supports file IDs or Google Drive/Docs URLs. Exports Google Workspace docs (Docs→text, Sheets→CSV, Slides→text). Content is truncated at 100 KB for large files.

### Profiles

```sh
personal-google-mcp profiles
```

Lists available profile names to stdout (no files created).

### Output Format

All CLI commands (except `profiles`) write two files:

- **Markdown** (`.md`) — Human-readable with YAML frontmatter containing `tool`, `profile`, `date`, and `params`
- **JSON sidecar** (`.json`) — Full structured data

Files are saved to `{output_dir}/{service}/YYYY-MM-DD-{name}.{md,json}`.

Stdout contains only the markdown file path. Errors and logs go to stderr.

## Prerequisites

1. A Google Cloud project with the required APIs enabled
2. OAuth 2.0 credentials (Desktop application type)

### Google Cloud Setup

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a project (or select an existing one)
3. Enable the required APIs under APIs & Services → Library:
   - **Google Classroom API**
   - **Google Drive API**
   - **Google Calendar API**
4. Go to APIs & Services → Credentials → Create Credentials → OAuth client ID
5. Select **Desktop app** as application type
6. Download the JSON and save it as `~/.config/personal-google-mcp/credentials.json`

## Multi-Profile Authentication

The server supports multiple Google accounts via named profiles.

### Authenticate

```sh
# Default profile
cargo run -- auth
# or with Nix: pgm-auth

# Named profile
PGM_PROFILE=work cargo run -- auth
```

This opens a browser for Google sign-in and saves tokens. Tokens auto-refresh on subsequent runs.

### File Layout

```
~/.config/personal-google-mcp/
├── credentials.json              # Shared OAuth client config
├── tokens.json                   # Default profile tokens
└── work/                         # Named profile
    ├── tokens.json
    └── cache/                    # Per-profile disk cache
```

At startup, the server discovers all authenticated profiles by scanning for directories containing `tokens.json`. Use `--profile` on CLI commands or the `profile` parameter on MCP tools to select an account.

## Build

```sh
# With Nix (recommended)
direnv allow   # or: nix develop
pgm-build      # or: cargo build --release

# Without Nix
cargo build --release
```

## Usage

### MCP Server (stdio)

```sh
cargo run -- run
```

### Claude Desktop

Add to your Claude Desktop config (`~/.config/claude/claude_desktop_config.json`):

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

### Testing with MCP Inspector

```sh
npx @modelcontextprotocol/inspector cargo run -- run
```

## Development

```sh
nix develop   # or: direnv allow
pgm-dev       # cargo run -- run
pgm-auth      # cargo run -- auth
pgm-test      # cargo test
pgm-build     # cargo build --release
```

## License

MIT
