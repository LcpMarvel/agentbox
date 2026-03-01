# AgentBox 实现规划

## Context

基于 prd.md 中的产品设计，从零开始构建 AgentBox —— 一个"PM2 for AI Agents"的本地 AI Agent 调度管理平台。当前目录仅有 prd.md，需要完整建立项目骨架。

目标：Phase 1 MVP —— 一行注册、定时跑、看得到。

---

## 项目结构

```
agent-box/
├── Cargo.toml                    # Workspace root
├── Cargo.lock
├── README.md
├── install.sh
├── .github/workflows/
│   ├── ci.yml                    # cargo test + clippy
│   └── release.yml               # cross-compile + GitHub Release
│
├── crates/
│   ├── agentbox-cli/             # 唯一 binary target
│   │   └── src/
│   │       ├── main.rs
│   │       └── cli/commands/     # register, list, run, schedule, logs, pause, resume, daemon, dashboard
│   │
│   ├── agentbox-core/            # 共享类型
│   │   └── src/
│   │       ├── types.rs          # AgentStatus, TriggerType, ScheduleConfig, IPC 消息
│   │       ├── error.rs
│   │       └── config.rs         # ~/.agentbox/config.toml 读写
│   │
│   ├── agentbox-db/              # 数据访问层
│   │   └── src/
│   │       ├── connection.rs     # SQLite 连接池 + migration
│   │       ├── migrations/       # m001_initial, m002_alerts
│   │       ├── models/           # Agent, Run, Log struct
│   │       └── repo/             # AgentRepo, RunRepo, LogRepo
│   │
│   ├── agentbox-daemon/          # 后台守护进程逻辑
│   │   └── src/
│   │       ├── daemon.rs         # double-fork，PID 文件
│   │       ├── scheduler/        # engine.rs, cron.rs, interval.rs
│   │       ├── executor/         # process.rs, supervisor.rs
│   │       ├── log_collector/    # stdout/stderr 异步采集
│   │       └── ipc/              # Unix socket server + JSON-RPC protocol
│   │
│   └── agentbox-web/             # 嵌入式 Web 服务
│       └── src/
│           ├── server.rs         # axum，port 9800
│           ├── api/              # REST handlers
│           └── assets.rs         # rust-embed 静态资源
│
└── dashboard/                    # 前端源码（Vue 3 + Vite + UnoCSS）
    ├── package.json
    ├── vite.config.ts
    └── src/
        ├── views/                # AgentList, AgentDetail, RunHistory
        ├── components/           # StatusBadge, LogViewer, ScheduleForm
        └── api/client.ts
```

---

## 技术选型

| 组件 | 选型 |
|------|------|
| 主语言 | Rust (workspace，单二进制输出) |
| CLI | clap 4 (derive feature) |
| 异步运行时 | tokio (full features) |
| 数据库 | rusqlite (bundled 静态链接) + r2d2 连接池 |
| IPC | Unix Domain Socket + JSON-RPC 2.0（路径：~/.agentbox/daemon.sock）|
| Web 框架 | axum 0.7 |
| 前端嵌入 | rust-embed 8 |
| 调度 | 自实现 BinaryHeap 最小堆 + cron crate |
| 进程管理 | tokio::process + nix (setpgid/killpg) |
| 前端 | Vue 3 + Vite + Pinia + UnoCSS |
| 输出格式化 | comfy-table + colored |

---

## 数据库 Schema（~/.agentbox/agentbox.db）

```sql
CREATE TABLE agents (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    name          TEXT NOT NULL UNIQUE,
    command       TEXT NOT NULL,
    working_dir   TEXT,
    env_vars      TEXT NOT NULL DEFAULT '{}',   -- JSON
    schedule_type TEXT NOT NULL DEFAULT 'manual', -- cron|interval|after|manual
    cron_expr     TEXT,
    interval_secs INTEGER,
    after_agent_id INTEGER REFERENCES agents(id),
    status        TEXT NOT NULL DEFAULT 'idle',  -- idle|running|paused|error
    paused        INTEGER NOT NULL DEFAULT 0,
    timeout_secs  INTEGER,
    max_retries   INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    last_run_at   TEXT,
    next_run_at   TEXT
);

CREATE TABLE runs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id     INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    status       TEXT NOT NULL DEFAULT 'running', -- running|success|failed|timeout|cancelled
    trigger_type TEXT NOT NULL,                   -- cron|interval|after|manual|api
    started_at   TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at     TEXT,
    duration_ms  INTEGER,
    exit_code    INTEGER,
    error_message TEXT,
    pid          INTEGER,
    retry_count  INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE logs (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id   INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    run_id     INTEGER NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    level      TEXT NOT NULL DEFAULT 'stdout',   -- stdout|stderr|system
    message    TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

SQLite 配置：WAL 模式，foreign_keys=ON，同步=NORMAL。

---

## CLI ↔ Daemon IPC 协议

Socket 路径：`~/.agentbox/daemon.sock`
协议：换行符分隔的 JSON-RPC 2.0

```
// 请求
{"jsonrpc":"2.0","id":1,"method":"agent.run","params":{"name":"stock-report"}}

// 响应
{"jsonrpc":"2.0","id":1,"result":{"run_id":42}}
```

**IPC 方法**：agent.register / agent.list / agent.run / agent.pause / agent.resume / agent.remove / agent.edit / logs.tail / runs.history / daemon.status / daemon.stop

CLI 连接策略：若 socket 不存在，自动在后台启动 daemon 并等待就绪（最多 3s）。

---

## 调度引擎设计

```
SchedulerEngine（独立 tokio task）
  ├── job_queue: BinaryHeap<ScheduledJob>  // 按 next_run_at 排序的最小堆
  ├── running_procs: HashMap<RunId, ProcessHandle>
  └── event_rx: mpsc::Receiver<SchedulerEvent>  // IPC 触发、进程退出、配置更新

主循环：tokio::select! {
    _ = sleep(until_next_job) => { 触发队列头部任务 }
    event = event_rx.recv()  => { 处理 IPC 命令 / 进程退出 }
}
```

子进程管理：
- `tokio::process::Command`，setpgid(0,0) 确保整组终止
- stdout/stderr 各起独立 tokio task 异步写入 SQLite
- 超时：tokio::select! 等待子进程 vs sleep(timeout)，超时后 killpg(SIGTERM) + 5s 宽限 + SIGKILL

---

## Web Dashboard REST API

```
GET  /api/agents               所有 agents + 状态
GET  /api/agents/:id/runs      运行历史（分页）
GET  /api/agents/:id/logs      最近日志
POST /api/agents/:id/run       手动触发
POST /api/agents/:id/pause     暂停
POST /api/agents/:id/resume    恢复
GET  /api/runs/:id/logs/stream SSE 实时日志流
GET  /api/dashboard/stats      汇总统计
```

前端通过 `rust-embed` 在编译时嵌入 `dashboard/dist/`，SPA fallback 返回 `index.html`。

---

## Phase 1 实现顺序（~12周）

| Sprint | 周 | 内容 |
|--------|---|------|
| 1 | 1-2 | Cargo workspace 骨架 + agentbox-core（类型/错误/配置）+ agentbox-db（schema + repo） |
| 2 | 3-4 | agentbox-cli 骨架（clap）+ IPC protocol + register/list 命令（纯 DB） |
| 3 | 5-6 | agentbox-daemon：double-fork + Unix socket server + agent.run 执行 |
| 4 | 7-8 | 调度引擎：BinaryHeap 主循环 + cron/interval 计算 + pause/resume |
| 5 | 9-10 | dashboard（Vue 3）+ agentbox-web（axum）+ rust-embed 打包 |
| 6 | 11-12 | macOS launchd 集成 + install.sh + GitHub Actions CI/CD + 端到端测试 |

---

## 关键依赖（Cargo workspace）

```toml
tokio = { version = "1.38", features = ["full"] }
clap = { version = "4.5", features = ["derive", "env"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rusqlite = { version = "0.31", features = ["bundled"] }  # 静态链接，单二进制关键
r2d2_sqlite = "0.24"
thiserror = "1"
anyhow = "1"
chrono = { version = "0.4", features = ["serde"] }
cron = "0.12"
axum = "0.7"
tower-http = { version = "0.5", features = ["cors"] }
rust-embed = { version = "8", features = ["axum"] }
nix = { version = "0.29", features = ["process", "signal"] }
dirs = "5"
comfy-table = "7"
colored = "2"
tracing = "0.1"
tracing-subscriber = "0.3"

[profile.release]
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

---

## macOS 集成

- `agentbox daemon install` 生成 LaunchAgent plist → `~/Library/LaunchAgents/com.agentbox.daemon.plist`
- `launchctl load` 实现开机自启
- Apple Silicon 优先，同时支持 x86_64（GitHub Actions 交叉编译）

---

## 验证方式

```bash
# 1. 单元测试
cargo test --workspace

# 2. 集成测试：完整流程
agentbox daemon start
agentbox register test-echo "echo hello world"
agentbox schedule test-echo "* * * * *"  # 每分钟
agentbox list                              # 应显示 idle 状态
agentbox run test-echo                    # 手动触发
agentbox logs test-echo                   # 应看到 "hello world"
agentbox pause test-echo
agentbox list                              # 应显示 paused
agentbox resume test-echo
agentbox dashboard                        # 打开 http://localhost:9800
agentbox remove test-echo
agentbox daemon stop

# 3. 二进制大小检查（release build 应 < 20MB）
cargo build --release
ls -lh target/release/agentbox
```

---

## 关键文件

- `crates/agentbox-core/src/types.rs` — 全系统数据契约（IPC 消息、枚举类型）
- `crates/agentbox-db/src/migrations/m001_initial.rs` — 核心表 DDL
- `crates/agentbox-daemon/src/scheduler/engine.rs` — 调度主循环
- `crates/agentbox-daemon/src/executor/process.rs` — 子进程 spawn 与日志采集
- `crates/agentbox-daemon/src/ipc/server.rs` — Unix socket + JSON-RPC 分发
- `crates/agentbox-web/src/assets.rs` — rust-embed 前端嵌入
- `dashboard/src/views/AgentList.vue` — Dashboard 主页面
