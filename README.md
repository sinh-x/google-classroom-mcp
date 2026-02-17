# Google Classroom MCP Server

A Rust [MCP](https://modelcontextprotocol.io/) server that provides read-only access to Google Classroom data — courses, announcements, assignments, and student submissions.

## Tools

| Tool | Parameters | Description |
|------|-----------|-------------|
| `courses` | — | List all courses for the authenticated user |
| `course_details` | `course_id` | Get course info + recent announcements (up to 20) |
| `assignments` | `course_id` | Get coursework + student submissions for the first 5 assignments |

## Prerequisites

1. A Google Cloud project with the **Classroom API** enabled
2. OAuth 2.0 credentials (Desktop application type)

### Google Cloud Setup

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a project (or select an existing one)
3. Enable the **Google Classroom API** under APIs & Services → Library
4. Go to APIs & Services → Credentials → Create Credentials → OAuth client ID
5. Select **Desktop app** as application type
6. Download the JSON and save it as `~/.config/google-classroom-mcp/credentials.json`

## Build

```sh
# With Nix (recommended)
direnv allow   # or: nix develop
gcm-build      # or: cargo build --release

# Without Nix
cargo build --release
```

## Authenticate

Run the auth command once to sign in with Google:

```sh
cargo run -- auth
# or in nix shell: gcm-auth
```

This opens a browser for Google sign-in and saves tokens to `~/.config/google-classroom-mcp/tokens.json`. Tokens auto-refresh on subsequent runs.

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
    "google-classroom": {
      "command": "/path/to/google-classroom-mcp",
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
gcm-dev       # cargo run -- run
gcm-auth      # cargo run -- auth
gcm-test      # cargo test
gcm-build     # cargo build --release
```

## License

MIT
