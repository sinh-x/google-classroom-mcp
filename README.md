# Personal Google MCP Server

A Rust [MCP](https://modelcontextprotocol.io/) server that provides access to personal Google services — currently Classroom, Drive, with Calendar and more planned.

## Tools

| Tool | Parameters | Description |
|------|-----------|-------------|
| `courses` | — | List all courses for the authenticated user |
| `course_details` | `course_id` | Get course info + recent announcements (up to 20) |
| `assignments` | `course_id` | Get coursework + student submissions for the first 5 assignments |
| `course_materials` | `course_id` | Posted resources (docs, links, videos) |
| `course_topics` | `course_id` | Topics (modules/sections) organizing content |
| `read_material` | `file_id_or_url` | Read Google Drive file content (Docs, Sheets, CSV) |

## Prerequisites

1. A Google Cloud project with the required APIs enabled
2. OAuth 2.0 credentials (Desktop application type)

### Google Cloud Setup

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a project (or select an existing one)
3. Enable the required APIs under APIs & Services → Library:
   - **Google Classroom API**
   - **Google Drive API**
4. Go to APIs & Services → Credentials → Create Credentials → OAuth client ID
5. Select **Desktop app** as application type
6. Download the JSON and save it as `~/.config/personal-google-mcp/credentials.json`

## Build

```sh
# With Nix (recommended)
direnv allow   # or: nix develop
pgm-build      # or: cargo build --release

# Without Nix
cargo build --release
```

## Authenticate

Run the auth command once to sign in with Google:

```sh
cargo run -- auth
# or in nix shell: pgm-auth
```

This opens a browser for Google sign-in and saves tokens to `~/.config/personal-google-mcp/tokens.json`. Tokens auto-refresh on subsequent runs.

## Usage

### Standalone (stdio)

```sh
cargo run -- run
```

### Claude Desktop

Add to your Claude Desktop config (`~/.config/claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "personal-google": {
      "command": "/path/to/personal-google-mcp",
      "args": ["run"]
    }
  }
}
```

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
