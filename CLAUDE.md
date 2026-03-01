# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is AgentBox

"PM2 for AI Agents" — a local daemon-based agent scheduler/runner written in Rust. Single binary includes CLI, daemon, web dashboard, and MCP server.

## Common Commands

All task running uses [just](https://github.com/casey/just). The dashboard **must build before Rust** because `rust-embed` embeds `dashboard/dist/` at compile time.

```bash
just setup          # First-time: git hooks + dashboard npm ci
just check          # Full CI: dashboard-build → fmt → clippy → test
just build          # Dev build (cargo build --workspace)
just release        # Release build (LTO enabled)
just dev            # Run daemon foreground (cargo run -- daemon start --foreground)
just fmt-fix        # Auto-fix formatting
just dashboard-build  # Build Vue frontend only
```

Individual checks:
```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

Pre-commit hook (`.githooks/pre-commit`) runs `just check` automatically.

## Architecture

```
CLI / MCP Server
       │
       │  Unix Socket (~/.agentbox/daemon.sock)
       │  JSON-RPC 2.0, newline-delimited
       ▼
    Daemon
    ├── Scheduler (BinaryHeap min-heap + cron + dependency chains)
    ├── Executor (sh -c <cmd>, process groups via setpgid, retry/timeout)
    ├── Log Collector (stdout/stderr → SQLite)
    ├── Alert Manager (webhook, telegram, macOS notifications)
    └── Web Server (axum on :9800, REST API + SSE + embedded SPA)
```

Data lives in `~/.agentbox/` (SQLite WAL mode, daemon.sock, daemon.pid).

## Crate Dependency Graph

```
agentbox-core    ← shared types, AgentBoxError (thiserror), IPC protocol types, config paths
agentbox-db      ← rusqlite + r2d2 pool, homegrown migrations, repository modules
agentbox-daemon  ← scheduler engine, process executor, IPC server, alert manager
agentbox-web     ← axum HTTP server, REST API, SSE, rust-embed serves dashboard/dist/
agentbox-cli     ← clap 4 binary, IPC client, auto-starts daemon if socket missing
agentbox-mcp     ← rmcp 0.12 stdio transport, all tools proxy through IPC to daemon
```

## Key Patterns

- **Error handling**: `thiserror` enums in `agentbox-core` for typed errors; `anyhow` in application code (daemon, CLI, web). DB repos return `Result<T, Box<dyn Error>>`, bridged to anyhow with `map_err`.
- **IPC protocol**: `IpcRequest`/`IpcResponse` in `agentbox-core/src/types.rs`. Methods are `agent.register`, `agent.list`, `agent.run`, `agent.schedule`, `logs.tail`, `runs.history`, `config.set`, `daemon.status`, etc.
- **Process isolation**: Each agent runs as `sh -c <command>` in a new process group (`pre_exec` + `nix::unistd::setpgid`). Timeout kills via `killpg(SIGTERM)` then `SIGKILL`.
- **Scheduler**: `tokio::sync::mpsc` channel (cap 256) for `SchedulerEvent`s. `tokio::select!` loop fires due jobs from the heap or handles events.
- **Dashboard embedding**: `rust-embed` bakes `dashboard/dist/` into the binary. Axum falls back to `index.html` for non-`/api` routes (SPA routing).
- **DB migrations**: Homegrown runner in `agentbox-db/src/migrations.rs` using a `schema_version` table.

## Dashboard (Vue 3)

```bash
cd dashboard && npm ci    # Install deps
npm run build             # Build to dist/
npm run dev               # Dev server with proxy to localhost:19898
```

Stack: Vue 3 (Composition API) + Vite 5 + UnoCSS (attribute mode). Routes: `/` (dashboard), `/agents` (list), `/agents/:id` (detail + SSE logs), `/alerts`.

## Branch Convention

This repo uses `master` as the default branch (not `main`). CI triggers on push/PR to `master`.
