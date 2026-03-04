# Toggly

A self-hosted feature toggle management platform inspired by LaunchDarkly and ConfigCat.

## Architecture

Toggly is organized as a Cargo workspace with three crates:

- **toggly-core** — Shared types, data models, and SQLite database layer
- **toggly-server** — Axum HTTP server hosting the Management API and Client Evaluation API
- **toggly-tui** — Terminal UI (ratatui) for managing feature toggles

### Data Model

- **Project** — Top-level organizational unit (e.g. "my-app")
- **Environment** — Deployment context within a project (e.g. "production", "staging"), each with a unique SDK key
- **Flag** — Feature flag definition scoped to a project, with support for boolean, string, number, and JSON types
- **Flag Environment State** — Per-environment configuration for each flag (enabled/disabled, default value, off value)

## Getting Started

### Prerequisites

- Rust 1.70+

### Running the Server

```bash
export TOGGLY_ADMIN_API_KEY="your-secret-admin-key"
cargo run --bin toggly-server
```

The server starts on `0.0.0.0:8080` by default. Configuration via environment variables:

- `TOGGLY_DB_PATH` — Path to SQLite database file (default: `toggly.db`)
- `TOGGLY_ADMIN_API_KEY` — (required) Admin API key for management endpoints
- `TOGGLY_HOST` — Bind address (default: `0.0.0.0`)
- `TOGGLY_PORT` — Bind port (default: `8080`)

### Running the TUI

```bash
export TOGGLY_ADMIN_API_KEY="your-secret-admin-key"
cargo run --bin toggly-tui
```

- `TOGGLY_SERVER_URL` — Server base URL (default: `http://localhost:8080`)
- `TOGGLY_ADMIN_API_KEY` — (required) Admin API key

### TUI Keybindings

- `j`/`k` or arrow keys — Navigate
- `Enter` — Select / drill in
- `Esc`/`q` — Go back / quit
- `c` — Create (project, environment, or flag)
- `d` — Delete
- `t` — Toggle flag enabled/disabled (on flag detail screen)
- `Tab` — Switch between environments and flags panes
- `r` — Refresh

## API Reference

### Management API (`/api/v1/...`)

Authenticated via `Authorization: Bearer <ADMIN_API_KEY>`.

**Projects**
- `POST /api/v1/projects` — Create project
- `GET /api/v1/projects` — List projects
- `GET /api/v1/projects/:id` — Get project
- `PUT /api/v1/projects/:id` — Update project
- `DELETE /api/v1/projects/:id` — Delete project

**Environments**
- `POST /api/v1/projects/:project_id/environments` — Create environment
- `GET /api/v1/projects/:project_id/environments` — List environments
- `DELETE /api/v1/projects/:project_id/environments/:id` — Delete environment

**Flags**
- `POST /api/v1/projects/:project_id/flags` — Create flag
- `GET /api/v1/projects/:project_id/flags` — List flags
- `GET /api/v1/projects/:project_id/flags/:key` — Get flag with all environment states
- `PUT /api/v1/projects/:project_id/flags/:key` — Update flag metadata
- `DELETE /api/v1/projects/:project_id/flags/:key` — Delete flag

**Flag State**
- `PUT /api/v1/projects/:project_id/flags/:key/environments/:env_id` — Update flag state

### Client Evaluation API (`/eval/v1/...`)

Authenticated via `Authorization: Bearer <SDK_KEY>` (the SDK key of the target environment).

- `GET /eval/v1/flags` — Get all flag values for the environment
- `GET /eval/v1/flags/:key` — Get a single flag's resolved value

**Example:**

```bash
curl -H "Authorization: Bearer sdk-xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx" \
  http://localhost:8080/eval/v1/flags

# { "flags": { "dark-mode": { "enabled": true, "value": true }, ... } }
```
