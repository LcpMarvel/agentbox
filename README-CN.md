# AgentBox

> **PM2 for AI Agents** — 专为个人开发者设计的本地 AI Agent 管理平台。

一行命令注册，定时自动跑，日志看得到。不需要 Docker，不需要配置文件，不需要云服务。

[English](./README.md)

## 为什么需要 AgentBox

你可能同时跑着好几个 AI Agent：投研分析、新闻监控、内容生成、代码维护……

但现实是：脚本散落各处，cron 和 launchd 混着用，日志不知道在哪，Agent 挂了几天都不知道。

AgentBox 解决这个问题：**一个统一的地方注册、调度、监控你所有的 Agent。**

| | AgentBox | cron | PM2 | n8n |
|---|----------|------|-----|-----|
| 注册方式 | 一行命令 | crontab 编辑 | 配置文件 | Web UI 拖拽 |
| Web Dashboard | ✅ 内置 | ❌ | ⚠️ 付费 | ✅ |
| 日志聚合 | ✅ | ❌ | ✅ | ✅ |
| 故障告警 | ✅ 多渠道 | ❌ | ⚠️ 基础 | ✅ |
| 安装复杂度 | 单二进制 | 系统自带 | npm install | Docker |
| 资源占用 | 极低 | 极低 | 低 | 中 |

## 安装

```bash
# 一键安装（自动检测系统架构，从 GitHub Releases 下载）
curl -fsSL https://raw.githubusercontent.com/LcpMarvel/agentbox/main/install.sh | sh

# 或从源码编译
cargo install --path crates/agentbox-cli

# 或手动编译
cargo build --release
cp target/release/agentbox ~/.local/bin/
```

### macOS 开机自启

```bash
agentbox daemon install     # 注册 launchd 服务，开机自动启动守护进程
agentbox daemon uninstall   # 移除 launchd 服务
```

## 3 分钟上手

```bash
# 1. 启动后台守护进程
agentbox daemon start

# 2. 注册一个 Agent — 任何终端命令都行
agentbox register morning-brief "claude -p '总结今日科技新闻要点，保存到 ~/briefs/'"

# 3. 设置每天早上 8 点执行
agentbox schedule morning-brief "0 8 * * *"

# 4. 手动跑一次看看效果
agentbox run morning-brief

# 5. 看日志
agentbox logs morning-brief

# 6. 再注册几个
agentbox register stock-scan "python ~/scripts/stock_scan.py"
agentbox schedule stock-scan "0 18 * * 1-5"

agentbox register repo-cleanup "cd ~/projects && bash cleanup.sh"
agentbox schedule repo-cleanup --every 7d

# 7. 看全局状态
agentbox list

# 8. 打开 Web Dashboard
agentbox dashboard
```

完事了。不需要建目录，不需要写配置文件，不需要懂 launchd plist 语法。

## 什么是 Agent

在 AgentBox 中，一个 Agent = **一个名字 + 一条命令**。

任何能在终端执行的命令都可以注册为 Agent：

```bash
# Claude CLI 做投研分析
agentbox register stock-report "claude -p '分析今日A股异动个股，生成报告保存到 ~/reports/'"

# Python 脚本做新闻监控
agentbox register news-monitor "python ~/scripts/news_monitor.py"

# Node.js 脚本做内容聚合
agentbox register digest "node ~/agents/digest/index.js"

# Shell 脚本做备份
agentbox register backup "bash ~/scripts/backup.sh"

# 容器化的 Agent
agentbox register my-agent "docker run --rm my-agent"
```

AgentBox 不关心命令里面是什么语言、什么框架、调用了什么 API。它只负责：**按时执行、采集日志、监控状态。**

## CLI 完整参考

### 注册与管理

```bash
agentbox register <name> <command>    # 注册一个 Agent
agentbox register <name> <command> -d ~/work  # 指定工作目录
agentbox register <name> <command> --timeout 300  # 超时 5 分钟自动终止
agentbox register <name> <command> --retry 3  # 失败最多重试 3 次
agentbox register <name> <command> --retry 3 --retry-delay 60 --retry-strategy exponential  # 指数退避重试

agentbox list                         # 查看所有 Agent 状态
agentbox run <name>                   # 手动触发一次
agentbox remove <name>                # 移除 Agent
```

### 调度

```bash
agentbox schedule <name> "0 18 * * *"    # Cron 表达式：每天 18 点
agentbox schedule <name> "*/30 * * * *"  # 每 30 分钟
agentbox schedule <name> --every 2h      # 固定间隔：每 2 小时
agentbox schedule <name> --every 30m     # 每 30 分钟
agentbox schedule <name> --manual        # 取消调度，改为手动
agentbox schedule <name> "0 9 * * *" --after data-fetch  # 依赖链：data-fetch 成功后触发
agentbox pause <name>                    # 暂停调度
agentbox resume <name>                   # 恢复调度
```

### 配置与告警

```bash
# 全局配置
agentbox config set max_concurrent 5      # 最大并发执行数

# 告警渠道
agentbox config alert.webhook https://hooks.slack.com/xxx   # 添加 Webhook 告警
agentbox config alert.telegram <bot_token> <chat_id>        # 添加 Telegram 告警
agentbox config alert.macos enable                          # 启用 macOS 通知中心
agentbox config alert.list                                  # 查看已配置的告警渠道
agentbox config alert.remove <id>                           # 移除告警渠道
```

### 日志与历史

```bash
agentbox logs <name>              # 查看最近 50 条日志
agentbox logs <name> -n 100       # 查看最近 100 条
agentbox logs --all               # 所有 Agent 的聚合日志
agentbox history <name>           # 查看运行历史
agentbox history <name> -n 50     # 最近 50 次运行
```

### 守护进程

```bash
agentbox daemon start             # 后台启动守护进程
agentbox daemon start --foreground  # 前台启动（调试用）
agentbox daemon stop              # 停止守护进程
agentbox daemon status            # 查看守护进程状态
agentbox daemon install           # 注册 macOS launchd 服务（开机自启）
agentbox daemon uninstall         # 移除 launchd 服务
```

### Web Dashboard

```bash
agentbox dashboard                # 打开 http://localhost:9800
```

### `agentbox list` 输出示例

```
┌──────────────┬────────────┬──────────────┬──────────┬────────────────────────┐
│ Name         │ Status     │ Schedule     │ Last Run │ Command                │
├──────────────┼────────────┼──────────────┼──────────┼────────────────────────┤
│ stock-report │ ✅ idle    │ 0 18 * * *   │ 2min ago │ claude -p '分析...'    │
│ news-monitor │ 🔄 running │ every 2h     │ running  │ python ~/scripts/ne…   │
│ kb-update    │ ❌ failed  │ 0 3 * * *    │ 5h ago   │ node ~/agents/kb/in…   │
│ code-review  │ ⏸ paused   │ manual       │ 3d ago   │ bash ~/scripts/revi…   │
└──────────────┴────────────┴──────────────┴──────────┴────────────────────────┘
```

## MCP 服务（AI 集成）

AgentBox 内置 [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) 服务，让 Claude Desktop、Cursor 等 AI 助手可以通过自然语言直接管理你的 Agent。

```bash
# 启动 MCP 服务（stdio 传输）
agentbox mcp
```

### 配置方法

将 AgentBox 添加到你的 MCP 客户端配置中：

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

### 可用工具

| 工具 | 说明 |
|------|------|
| `list_agents` | 列出所有 Agent 及状态、调度、上次运行 |
| `register_agent` | 注册新 Agent（名称 + 命令） |
| `run_agent` | 手动触发运行 |
| `schedule_agent` | 设置 cron / 固定间隔 / 依赖链调度 |
| `pause_agent` | 暂停自动调度 |
| `resume_agent` | 恢复已暂停的调度 |
| `remove_agent` | 永久删除 Agent 及其历史 |
| `get_agent_logs` | 获取最近的 stdout/stderr 日志 |
| `get_run_history` | 查看运行历史（状态、耗时） |
| `get_dashboard_stats` | 全局统计：总数、运行中、错误数、成功率 |
| `get_config` / `set_config` | 读写全局配置 |
| `manage_alerts` | 添加、列出、删除告警渠道 |

### MCP Resources

| URI | 说明 |
|-----|------|
| `agentbox://agents` | 所有 Agent 列表（JSON） |
| `agentbox://agents/{name}` | 某个 Agent 的详细信息 |
| `agentbox://agents/{name}/logs` | 某个 Agent 的最近日志 |

### 使用示例

配置完成后，直接用自然语言对话：

> "注册一个叫 `daily-report` 的 Agent，命令是 `python ~/scripts/report.py`，工作目录 `~/projects/reports`，每天早上 9 点跑"

> "看看最近有哪些 Agent 失败了"

> "暂停 stock-scan"

## 架构

AgentBox 是一个 Rust 编写的单二进制程序，包含三个部分：

```
┌─────────────────────────────────────────────────┐
│                   agentbox CLI                   │
│  register · list · run · schedule · logs · ...   │
│  config · dashboard · daemon install/uninstall   │
└──────────────────────┬──────────────────────────┘
                       │
              ┌────────┴────────┐
              │                 │
         CLI 命令         agentbox mcp
              │          (MCP 服务, stdio)
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

### 项目结构

```
crates/
├── agentbox-core/     # 共享类型、错误、配置路径
├── agentbox-db/       # SQLite 连接池 + migrations + repos（Agent/Run/Log/Alert/Config）
├── agentbox-daemon/   # 守护进程、调度引擎、进程执行器、IPC 服务、告警管理
├── agentbox-cli/      # CLI 入口（clap 4），输出 agentbox 二进制
├── agentbox-web/      # axum Web 服务 + REST API + SSE + 前端 SPA 嵌入
└── agentbox-mcp/      # MCP 服务（stdio 传输，12 个工具，3 个资源）

dashboard/             # Vue 3 + Vite + UnoCSS 前端，编译后通过 rust-embed 嵌入二进制
```

### 技术选型

| 组件 | 选型 | 理由 |
|------|------|------|
| 语言 | Rust | 低资源占用，单二进制分发，无运行时依赖 |
| CLI | clap 4 | Rust 生态最成熟 |
| 异步 | tokio | 全特性异步运行时 |
| 数据库 | rusqlite (bundled) | 零配置，静态链接，单文件 |
| 调度 | BinaryHeap + cron crate | 自实现最小堆，无系统依赖 |
| 进程管理 | tokio::process + nix | setpgid 确保进程组可靠终止 |
| Web | axum 0.8 | 轻量异步 Web 框架 |
| 前端 | Vue 3 + Vite + UnoCSS | 编译后通过 rust-embed 嵌入二进制 |
| IPC | Unix Domain Socket | JSON-RPC 2.0，换行分隔 |
| MCP | rmcp 0.12 | stdio 传输，MCP 协议 2025-03-26 |

### 数据存储

所有数据存于 `~/.agentbox/` 目录，不发送任何数据到云端：

- **agents** — 注册信息：名称、命令、调度规则、当前状态、重试配置
- **runs** — 运行记录：开始/结束时间、退出码、耗时、触发方式
- **logs** — 结构化日志：Agent、运行批次、级别 (stdout/stderr/system)、时间戳
- **alert_channels** — 告警渠道配置（Webhook / Telegram / macOS）
- **alert_history** — 告警发送历史
- **config** — 全局键值配置（并发数等）

## REST API

守护进程内嵌 Web 服务，默认监听 `localhost:9800`。

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/agents` | 列出所有 Agent |
| GET | `/api/agents/:id/runs` | 查看运行历史 |
| GET | `/api/agents/:id/logs?q=keyword&level=stderr` | 日志搜索（支持关键词+级别过滤）|
| GET | `/api/dashboard/stats` | Dashboard 统计数据 |
| GET | `/api/alerts` | 告警历史 |
| POST | `/api/agents/:id/run` | 手动触发运行 |
| POST | `/api/agents/:id/pause` | 暂停 Agent |
| POST | `/api/agents/:id/resume` | 恢复 Agent |
| POST | `/api/agents/trigger/:name` | Webhook 触发（按名称）|
| GET | `/api/runs/:id/logs/stream` | SSE 实时日志流 |

## 开发

```bash
# 编译
cargo build --workspace

# 运行测试
cargo test --workspace

# Release 编译（开启 LTO，体积更小）
cargo build --release

# 查看二进制大小
ls -lh target/release/agentbox

# 构建前端 Dashboard（需要 Node.js）
cd dashboard && npm install && npm run build
```

### CI/CD

项目使用 GitHub Actions：

- **CI** (`ci.yml`): 每次 push/PR 自动运行 `cargo fmt --check`、`cargo clippy`、`cargo test`、`npm run build`（dashboard）
- **Release** (`release.yml`): 打 `v*` tag 自动交叉编译三平台（macOS aarch64/x86_64 + Linux x86_64），上传到 GitHub Releases

## License

MIT
