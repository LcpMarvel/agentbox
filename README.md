# AgentBox

> **PM2 for AI Agents** — A local AI agent management platform designed for individual developers.

Register with one command, schedule automatic runs, view logs anytime. No Docker, no config files, no cloud services.

[中文文档](./README-CN.md)

## Why AgentBox

You probably have multiple AI agents running: research analysis, news monitoring, content generation, code maintenance…

But in reality: scripts are scattered everywhere, cron and launchd are mixed together, logs are nowhere to be found, and you don't even notice when an agent has been down for days.

AgentBox solves this: **one unified place to register, schedule, and monitor all your agents.**

| | AgentBox | cron | PM2 | n8n |
|---|----------|------|-----|-----|
| Registration | One command | Edit crontab | Config file | Web UI drag & drop |
| Web Dashboard | ✅ Built-in | ❌ | ⚠️ Paid | ✅ |
| Log Aggregation | ✅ | ❌ | ✅ | ✅ |
| Failure Alerts | ✅ Multi-channel | ❌ | ⚠️ Basic | ✅ |
| Install Complexity | Single binary | Built-in | npm install | Docker |
| Resource Usage | Very low | Very low | Low | Medium |

## Installation

```bash
# One-line install (auto-detects OS/arch, downloads from GitHub Releases)
curl -fsSL https://raw.githubusercontent.com/LcpMarvel/agentbox/main/install.sh | sh

# Or build from source
cargo install --path crates/agentbox-cli

# Or manual build
cargo build --release
cp target/release/agentbox ~/.local/bin/
```

### macOS Auto-Start

```bash
agentbox daemon install     # Register launchd service, auto-start on login
agentbox daemon uninstall   # Remove launchd service
```

## Quick Start

```bash
# 1. Start the background daemon
agentbox daemon start

# 2. Register an agent — any terminal command works
agentbox register morning-brief "claude -p 'Summarize today tech news, save to ~/briefs/'"

# 3. Schedule it to run daily at 8am
agentbox schedule morning-brief "0 8 * * *"

# 4. Run it manually to test
agentbox run morning-brief

# 5. View logs
agentbox logs morning-brief

# 6. Register more agents
agentbox register stock-scan "python ~/scripts/stock_scan.py"
agentbox schedule stock-scan "0 18 * * 1-5"

agentbox register repo-cleanup "cd ~/projects && bash cleanup.sh"
agentbox schedule repo-cleanup --every 7d

# 7. Check global status
agentbox list

# 8. Open Web Dashboard
agentbox dashboard
```

That's it. No directories to create, no config files to write, no launchd plist syntax to learn.

## What is an Agent

In AgentBox, an Agent = **a name + a command**.

Any command that runs in a terminal can be registered as an Agent:

```bash
# Claude CLI for research analysis
agentbox register stock-report "claude -p 'Analyze stock movements, save report to ~/reports/'"

# Python script for news monitoring
agentbox register news-monitor "python ~/scripts/news_monitor.py"

# Node.js script for content aggregation
agentbox register digest "node ~/agents/digest/index.js"

# Shell script for backup
agentbox register backup "bash ~/scripts/backup.sh"

# Containerized agent
agentbox register my-agent "docker run --rm my-agent"
```

AgentBox doesn't care what language, framework, or API is inside the command. It only handles: **scheduled execution, log collection, and status monitoring.**

## CLI Reference

### Registration & Management

```bash
agentbox register <name> <command>    # Register an agent
agentbox register <name> <command> -d ~/work  # Set working directory
agentbox register <name> <command> --timeout 300  # Auto-kill after 5 minutes
agentbox register <name> <command> --retry 3  # Retry up to 3 times on failure
agentbox register <name> <command> --retry 3 --retry-delay 60 --retry-strategy exponential  # Exponential backoff

agentbox list                         # List all agents with status
agentbox run <name>                   # Trigger a manual run
agentbox remove <name>                # Remove an agent
```

### Scheduling

```bash
agentbox schedule <name> "0 18 * * *"    # Cron expression: daily at 18:00
agentbox schedule <name> "*/30 * * * *"  # Every 30 minutes
agentbox schedule <name> --every 2h      # Fixed interval: every 2 hours
agentbox schedule <name> --every 30m     # Every 30 minutes
agentbox schedule <name> --manual        # Remove schedule, manual-only
agentbox schedule <name> "0 9 * * *" --after data-fetch  # Dependency chain: run after data-fetch succeeds
agentbox pause <name>                    # Pause scheduling
agentbox resume <name>                   # Resume scheduling
```

### Configuration & Alerts

```bash
# Global configuration
agentbox config set max_concurrent 5      # Max concurrent executions

# Alert channels
agentbox config alert.webhook https://hooks.slack.com/xxx   # Add Webhook alert
agentbox config alert.telegram <bot_token> <chat_id>        # Add Telegram alert
agentbox config alert.macos enable                          # Enable macOS Notification Center
agentbox config alert.list                                  # List configured alert channels
agentbox config alert.remove <id>                           # Remove an alert channel
```

### Logs & History

```bash
agentbox logs <name>              # View last 50 log entries
agentbox logs <name> -n 100       # View last 100 entries
agentbox logs --all               # Aggregated logs from all agents
agentbox history <name>           # View run history
agentbox history <name> -n 50     # Last 50 runs
```

### Daemon

```bash
agentbox daemon start             # Start daemon in background
agentbox daemon start --foreground  # Start in foreground (for debugging)
agentbox daemon stop              # Stop daemon
agentbox daemon status            # Check daemon status
agentbox daemon install           # Register macOS launchd service (auto-start)
agentbox daemon uninstall         # Remove launchd service
```

### Web Dashboard

```bash
agentbox dashboard                # Open http://localhost:9800
```

### `agentbox list` Example Output

```
┌──────────────┬────────────┬──────────────┬──────────┬────────────────────────┐
│ Name         │ Status     │ Schedule     │ Last Run │ Command                │
├──────────────┼────────────┼──────────────┼──────────┼────────────────────────┤
│ stock-report │ ✅ idle    │ 0 18 * * *   │ 2min ago │ claude -p 'Analyze…'   │
│ news-monitor │ 🔄 running │ every 2h     │ running  │ python ~/scripts/ne…   │
│ kb-update    │ ❌ failed  │ 0 3 * * *    │ 5h ago   │ node ~/agents/kb/in…   │
│ code-review  │ ⏸ paused   │ manual       │ 3d ago   │ bash ~/scripts/revi…   │
└──────────────┴────────────┴──────────────┴──────────┴────────────────────────┘
```

## MCP Server (AI Integration)

AgentBox includes a built-in [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) server, allowing AI assistants like Claude Desktop, Cursor, and other MCP clients to manage your agents via natural language.

```bash
# Start the MCP server (stdio transport)
agentbox mcp
```

### Configuration

Add AgentBox to your MCP client configuration:

**Claude Desktop** (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "agentbox": {
      "command": "agentbox",
      "args": ["mcp"]
    }
  }
}
```

**Cursor** (`.cursor/mcp.json`):

```json
{
  "mcpServers": {
    "agentbox": {
      "command": "agentbox",
      "args": ["mcp"]
    }
  }
}
```

### Available Tools

| Tool | Description |
|------|-------------|
| `list_agents` | List all registered agents with status, schedule, and last run |
| `register_agent` | Register a new agent (name + shell command) |
| `run_agent` | Manually trigger an agent run |
| `schedule_agent` | Set cron / interval / dependency-based schedule |
| `pause_agent` | Pause an agent's automatic schedule |
| `resume_agent` | Resume a paused agent |
| `remove_agent` | Permanently remove an agent and its history |
| `get_agent_logs` | Get recent stdout/stderr logs |
| `get_run_history` | View past executions with status and duration |
| `get_dashboard_stats` | Global stats: total agents, running, errors, success rate |
| `get_config` / `set_config` | Read/write global configuration |
| `manage_alerts` | Add, list, or remove alert channels |

### MCP Resources

| URI | Description |
|-----|-------------|
| `agentbox://agents` | List of all agents (JSON) |
| `agentbox://agents/{name}` | Details for a specific agent |
| `agentbox://agents/{name}/logs` | Recent logs for a specific agent |

### Example Usage

Once configured, just talk to your AI:

> "Register a new agent called `daily-report` that runs `python ~/scripts/report.py` in `~/projects/reports`, schedule it every day at 9am"

> "Show me which agents failed recently"

> "Pause the stock-scan agent"

## Architecture

AgentBox is a single binary written in Rust, consisting of three parts:

```
┌─────────────────────────────────────────────────┐
│                   agentbox CLI                   │
│  register · list · run · schedule · logs · ...   │
│  config · dashboard · daemon install/uninstall   │
└──────────────────────┬──────────────────────────┘
                       │
              ┌────────┴────────┐
              │                 │
         CLI commands     agentbox mcp
              │          (MCP Server, stdio)
              │                 │
              └────────┬────────┘
                       │ Unix Socket (JSON-RPC 2.0)
                       │ ~/.agentbox/daemon.sock
┌──────────────────────▼──────────────────────────┐
│                 agentbox daemon                   │
│  ┌────────────┐ ┌──────────┐ ┌────────────────┐ │
│  │ Scheduler  │ │ Executor │ │ Log Collector  │ │
│  │ (BinaryHeap│ │ (tokio   │ │ (stdout/stderr │ │
│  │ + cron +   │ │ process +│ │  → SQLite)     │ │
│  │ dependency │ │ retry +  │ │                │ │
│  │ chain)     │ │ timeout) │ │                │ │
│  └────────────┘ └──────────┘ └────────────────┘ │
│  ┌──────────────┐ ┌────────────────────────────┐ │
│  │ Alert Manager│ │  Web Dashboard (axum:9800) │ │
│  │ (webhook,   │ │  REST API + SSE + SPA      │ │
│  │  telegram,  │ │  Vue 3 (rust-embed)        │ │
│  │  macos)     │ │                            │ │
│  └──────────────┘ └────────────────────────────┘ │
└──────────────────────┬──────────────────────────┘
                       │
          ┌────────────▼────────────┐
          │   ~/.agentbox/          │
          │   ├── agentbox.db       │  SQLite (WAL mode)
          │   ├── daemon.sock       │  IPC socket
          │   └── daemon.pid        │  PID file
          └─────────────────────────┘
```

### Workspace Structure

```
crates/
├── agentbox-core/     # Shared types, errors, config paths
├── agentbox-db/       # SQLite connection pool + migrations + repos (Agent/Run/Log/Alert/Config)
├── agentbox-daemon/   # Daemon, scheduler, process executor, IPC server, alert manager
├── agentbox-cli/      # CLI entry point (clap 4), outputs `agentbox` binary
├── agentbox-web/      # axum web server + REST API + SSE + embedded SPA
└── agentbox-mcp/      # MCP server (stdio transport, 12 tools, 3 resources)

dashboard/             # Vue 3 + Vite + UnoCSS frontend, compiled into binary via rust-embed
```

### Tech Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | Rust | Low resource usage, single binary distribution, no runtime dependencies |
| CLI | clap 4 | Most mature CLI library in the Rust ecosystem |
| Async | tokio | Full-featured async runtime |
| Database | rusqlite (bundled) | Zero config, statically linked, single file |
| Scheduler | BinaryHeap + cron crate | Custom min-heap, no system dependencies |
| Process Mgmt | tokio::process + nix | setpgid for reliable process group termination |
| Web | axum 0.8 | Lightweight async web framework |
| Frontend | Vue 3 + Vite + UnoCSS | Compiled and embedded into binary via rust-embed |
| IPC | Unix Domain Socket | JSON-RPC 2.0, newline-delimited |
| MCP | rmcp 0.12 | stdio transport, MCP protocol 2025-03-26 |

### Data Storage

All data is stored in `~/.agentbox/`. No data is sent to the cloud.

- **agents** — Registration info: name, command, schedule rules, current status, retry config
- **runs** — Run records: start/end time, exit code, duration, trigger type
- **logs** — Structured logs: agent, run batch, level (stdout/stderr/system), timestamp
- **alert_channels** — Alert channel config (Webhook / Telegram / macOS)
- **alert_history** — Alert delivery history
- **config** — Global key-value config (concurrency limits, etc.)

## REST API

The daemon embeds a web server, listening on `localhost:9800` by default.

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/agents` | List all agents |
| GET | `/api/agents/:id/runs` | View run history |
| GET | `/api/agents/:id/logs?q=keyword&level=stderr` | Search logs (keyword + level filter) |
| GET | `/api/dashboard/stats` | Dashboard statistics |
| GET | `/api/alerts` | Alert history |
| POST | `/api/agents/:id/run` | Trigger a manual run |
| POST | `/api/agents/:id/pause` | Pause an agent |
| POST | `/api/agents/:id/resume` | Resume an agent |
| POST | `/api/agents/trigger/:name` | Webhook trigger (by name) |
| GET | `/api/runs/:id/logs/stream` | SSE real-time log stream |

## Development

This project uses [just](https://github.com/casey/just) as a command runner. Install with `cargo install just` or `brew install just`.

```bash
# First-time setup (git hooks + dashboard deps)
just setup

# Run all CI checks (fmt + clippy + test + dashboard build)
just check

# Individual checks
just fmt          # Format check
just fmt-fix      # Auto-fix formatting
just clippy       # Lint
just test         # Run tests

# Build
just build        # Dev build
just release      # Release build (LTO enabled, smaller binary)

# Run daemon in foreground (for dev)
just dev

# Clean all build artifacts
just clean
```

### CI/CD

The project uses GitHub Actions:

- **CI** (`ci.yml`): Runs `cargo fmt --check`, `cargo clippy`, `cargo test`, and `npm run build` (dashboard) on every push/PR
- **Release** (`release.yml`): On `v*` tags, cross-compiles for 3 platforms (macOS aarch64/x86_64 + Linux x86_64) and uploads to GitHub Releases

A pre-commit hook (via `.githooks/`) runs `just check` before each commit to catch issues locally. It is set up automatically by `just setup`.

## License

MIT
