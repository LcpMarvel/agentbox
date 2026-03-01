# AgentBox — 个人本地 AI Agent 管理平台

> 一个面向个人开发者和小团队的本地 AI Agent 调度、监控和管理工具。  
> 让你像管理容器一样管理你的 AI Agent，而不是靠记忆力。

---

## 一、项目背景

### 问题

AI Agent 正在成为个人开发者的核心生产力工具。一个典型的开发者可能同时运行多个 Agent：投研数据定时抓取、政策新闻监控、内容生成与发布、代码仓库自动维护、个人知识库定期更新等等。

但目前没有一个好用的工具来管理这些 Agent。现实情况是：

- **脚本散落各处**：不同目录、不同语言、不同配置方式
- **调度方式混乱**：cron、launchd、手动执行混着用
- **日志各自为政**：有的写文件、有的打 stdout、有的根本没日志
- **故障无人知晓**：Agent 挂了可能几天都不知道
- **新增 Agent 无规范**：每次都是复制粘贴，越来越乱

### 市场空白

| 层级 | 现有方案 | 问题 |
|------|---------|------|
| 进程管理 | PM2 / Supervisord | 不理解 Agent 概念，只是通用进程管理 |
| Agent 编排 | CrewAI / LangGraph | 解决的是一次任务内多 Agent 协作，不管生命周期 |
| 工作流平台 | n8n / Dify | 太重，个人开发者用不上全套功能 |
| 系统调度 | launchd / cron | 原始、无 UI、无监控、管理体验差 |

**AgentBox 填补的空白**：一个轻量的、面向 AI Agent 场景的本地调度管理器，带 Web Dashboard、日志聚合和故障告警。

---

## 二、产品定位

### 一句话定义

**"PM2 for AI Agents"** — 专为个人开发者设计的本地 AI Agent 管理平台。

### 目标用户

- 同时运行 3-20 个本地 AI Agent 的个人开发者
- 有自动化需求的独立创客、量化投资者、内容创作者
- 不想搭 Kubernetes 但又需要比 cron 更好的管理方式的人

### 设计原则

1. **一行注册**：`agentbox register <name> <command>` 就完事了，不需要建目录、写配置文件
2. **命令即 Agent**：任何能在终端执行的命令都可以成为一个 Agent
3. **轻量优先**：单二进制或单命令安装，不依赖 Docker/K8s
4. **CLI 优先 + Web UI**：开发者日常用 CLI，偶尔打开 Dashboard 看全局状态
5. **本地优先**：所有数据存本地，不依赖云服务，保护隐私

---

## 三、核心概念

### 什么是一个 Agent

在 AgentBox 中，一个 Agent = **一个名字 + 一条命令**。

任何能在终端执行的命令都可以注册为 Agent：

| 示例 | 说明 |
|------|------|
| `claude -p "分析今日A股异动个股，生成报告保存到 ~/reports/"` | Claude CLI 做投研分析 |
| `python ~/scripts/news_monitor.py` | Python 脚本做新闻监控 |
| `node ~/agents/digest/index.js` | Node.js 脚本做内容聚合 |
| `bash ~/scripts/backup.sh` | Shell 脚本做备份 |
| `cd ~/project && npm run analyze` | 进入目录后执行 |
| `docker run --rm my-agent` | 容器化的 Agent |

AgentBox 不关心命令里面是什么语言、什么框架、调用了什么 API。它只负责：按时执行、采集日志、监控状态、失败告警。

---

## 四、核心功能

### 4.1 注册与管理

**注册一个 Agent，只需要一行命令：**

`agentbox register <name> <command>`

注册后，通过 `agentbox schedule` 设置调度规则，Agent 就会按计划自动运行。

**完整 CLI：**

| 命令 | 说明 |
|------|------|
| `agentbox register <name> <command>` | 注册一个 Agent |
| `agentbox list` | 查看所有 Agent 状态 |
| `agentbox run <name>` | 手动触发一次 |
| `agentbox schedule <name> <cron>` | 设置调度规则 |
| `agentbox pause <name>` | 暂停调度 |
| `agentbox resume <name>` | 恢复调度 |
| `agentbox logs <name>` | 查看日志 |
| `agentbox logs --all` | 所有 Agent 的聚合日志流 |
| `agentbox history <name>` | 查看运行历史 |
| `agentbox edit <name>` | 修改命令或配置 |
| `agentbox remove <name>` | 移除 Agent |
| `agentbox daemon start/stop` | 启动/停止后台守护进程 |
| `agentbox dashboard` | 打开 Web Dashboard |
| `agentbox mcp` | 启动 MCP Server（供 AI 客户端调用） |

**使用示例：**

```
# 注册
agentbox register stock-report "claude -p '分析今日A股异动个股，生成报告保存到 ~/reports/'"

# 设置每天下午6点执行
agentbox schedule stock-report "0 18 * * *"

# 看看有哪些 Agent
agentbox list

# 手动跑一次试试
agentbox run stock-report

# 看日志
agentbox logs stock-report --tail 20
```

**`agentbox list` 输出示例：**

| Name | Status | Schedule | Last Run | Duration |
|------|--------|----------|----------|----------|
| stock-report | ✅ idle | daily 18:00 | 2min ago | 45s |
| news-monitor | 🔄 running | */2h | running... | — |
| kb-update | ❌ failed | daily 03:00 | 5h ago | 12s |
| code-review | ⏸ paused | weekly Mon 09:00 | 3 days ago | 2m30s |

### 4.2 调度选项

通过 `agentbox schedule` 或 `agentbox edit` 设置：

| 选项 | 说明 | 示例 |
|------|------|------|
| Cron 表达式 | 标准 cron 语法 | `"0 18 * * *"` 每天18点 |
| 固定间隔 | 从注册时刻开始 | `--every 30m` |
| 依赖触发 | 另一个 Agent 完成后触发 | `--after stock-report` |
| 手动 | 不自动调度 | `--manual`（默认） |

额外能力：

- **超时终止**：`--timeout 300` 超过5分钟自动杀死
- **失败重试**：`--retry 3` 失败后最多重试3次
- **并发控制**：同一时间最多运行 N 个 Agent，避免资源争抢
- **休眠补跑**：macOS 休眠期间错过的任务，唤醒后自动补跑

### 4.3 日志聚合

所有 Agent 的 stdout / stderr 统一采集，自动持久化存储。

- 按 Agent、时间范围、关键词查询
- 实时日志流（`agentbox logs <name> --follow`）
- 聚合日志（`agentbox logs --all`）查看所有 Agent 交叉时间线
- 自动日志轮转，按天数或总大小清理旧日志
- Web Dashboard 中可直接查看和搜索

### 4.4 告警通知

| 告警类型 | 说明 |
|---------|------|
| 失败告警 | Agent 运行失败（非零退出码） |
| 超时告警 | 运行时间超过设定阈值 |
| 静默告警 | Agent 按计划应该运行但没有运行 |
| 恢复通知 | 之前失败的 Agent 重新成功运行 |

通知渠道：

| 渠道 | 设置方式 |
|------|---------|
| Telegram Bot | `agentbox config alert.telegram <bot_token> <chat_id>` |
| Webhook | `agentbox config alert.webhook <url>` |
| 企业微信/飞书 | `agentbox config alert.wecom <webhook_url>` |
| Email | `agentbox config alert.email <smtp_config>` |
| macOS 系统通知 | 默认开启 |

### 4.5 Web Dashboard

轻量的本地 Web UI（默认 `http://localhost:9800`），通过 `agentbox dashboard` 打开：

| 页面 | 内容 |
|------|------|
| 总览 | 所有 Agent 状态卡片、今日运行次数、成功率 |
| Agent 详情 | 运行历史、日志、配置查看 |
| 时间线 | 甘特图形式展示各 Agent 运行时段，直观发现调度冲突 |
| 设置 | 全局配置、通知渠道管理 |

### 4.6 MCP Server（AI 集成）

通过 `agentbox mcp` 启动 MCP (Model Context Protocol) Server，让 AI 客户端可以直接管理 AgentBox。用户用自然语言就能完成 Agent 注册、调度、诊断等全部操作。

**运行方式**：独立 stdio 进程，走 MCP 标准的 stdin/stdout JSON-RPC 传输。

**配置示例**（Claude Desktop / Cursor / VS Code Copilot）：

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

**暴露能力**：

| 类型 | 名称 | 说明 |
|------|------|------|
| Tool | `list_agents` | 查看所有 Agent 状态 |
| Tool | `register_agent` | 注册新 Agent |
| Tool | `run_agent` | 手动触发一次 |
| Tool | `schedule_agent` | 设置调度（AI 将自然语言转为 cron 表达式） |
| Tool | `pause_agent` / `resume_agent` | 暂停/恢复调度 |
| Tool | `remove_agent` | 删除 Agent |
| Tool | `get_agent_logs` | 查看日志 |
| Tool | `get_run_history` | 查看运行历史 |
| Tool | `get_dashboard_stats` | 全局统计 |
| Tool | `get_config` / `set_config` | 配置管理 |
| Tool | `manage_alerts` | 告警渠道管理 |
| Resource | `agentbox://agents` | Agent 列表数据源 |
| Resource | `agentbox://agents/{name}` | 单个 Agent 详情 |
| Resource | `agentbox://agents/{name}/logs` | Agent 日志流 |

**典型交互**：

> 用户："帮我注册一个 agent，每天下午6点扫描A股异动"  
> AI 自动调用 `register_agent` + `schedule_agent(cron="0 18 * * *")`

> 用户："最近 stock-scan 跑得怎么样？"  
> AI 自动调用 `get_run_history` + `get_agent_logs`

---

## 五、技术架构

### 项目仓库

GitHub: `github.com/anthropic/agentbox`（待定）

仓库结构：

| 目录 | 说明 |
|------|------|
| `src/` | Rust 源码 |
| `dashboard/` | Web Dashboard 前端源码 |
| `install.sh` | 安装脚本 |
| `.github/workflows/` | CI/CD，自动编译 + 发布到 Releases |
| `README.md` | 项目首页 |

### 整体设计

AgentBox 是一个 Rust 编写的单二进制程序，包含三个部分：

1. **CLI**：用户交互入口，所有操作通过 `agentbox <command>` 完成
2. **Daemon**：后台守护进程，负责调度、进程管理、日志采集、告警
3. **Web Dashboard**：嵌入式前端，编译后打包进二进制，无需单独部署
4. **MCP Server**：通过 `agentbox mcp` 启动，暴露 AgentBox 全部能力为 MCP Tools/Resources，供 AI 客户端调用

### 技术选型

| 组件 | 选型 | 理由 |
|------|------|------|
| 主语言 | **Rust** | 低资源占用，单二进制分发，无运行时依赖，长期运行稳定 |
| CLI 框架 | clap | Rust 生态最成熟的 CLI 库 |
| 调度引擎 | 内置 cron parser | 不依赖系统 cron，跨平台一致 |
| 数据存储 | SQLite (rusqlite) | 零配置，单文件，性能足够 |
| Web Dashboard | 前端嵌入二进制 | 编译时打包静态资源，无需单独部署 |
| 进程管理 | OS native (fork/exec) | 直接管理子进程，不引入额外层 |
| MCP 协议 | rmcp (Rust MCP SDK) | 标准 MCP 实现，stdio 传输 |

### 数据存储

所有数据存于 `~/.agentbox/` 目录：

| 内容 | 说明 |
|------|------|
| `agentbox.db` | SQLite 数据库，存储 Agent 注册信息、运行记录、日志 |
| `config.yaml` | 全局配置（通知渠道、并发数、日志策略等） |

核心数据表：

| 表 | 说明 |
|------|------|
| agents | Agent 注册信息：名称、命令、调度规则、当前状态 |
| runs | 运行记录：开始/结束时间、退出码、耗时、触发方式、错误信息 |
| logs | 结构化日志：Agent、运行批次、级别、内容、时间戳 |

---

## 六、实现路线图

### Phase 1：MVP（4 周）

核心目标：**一行注册、定时跑、看得到**

- CLI 框架（register / list / run / schedule / logs / pause / resume）
- 基础调度引擎（cron + interval）
- 子进程管理（启动、超时终止、退出码捕获）
- 日志采集（stdout/stderr → SQLite）
- Daemon 模式 + macOS launchd 集成（开机自启）
- GitHub Actions CI/CD：自动编译多平台二进制，发布到 Releases
- install.sh 安装脚本

### Phase 2：MCP Server + 可观测性（3 周）

核心目标：**AI 可管理 + 知道 Agent 们在干什么**

- MCP Server（agentbox-mcp crate，stdio 传输，12 个 Tools + 3 个 Resources）
- AI 客户端一行配置即可用自然语言管理 Agent
- Web Dashboard（Agent 列表、状态、运行历史）
- 日志查看和搜索（Web UI）
- 运行历史时间线视图
- 基础告警（失败 → Telegram / Webhook / macOS 通知）

### Phase 3：高级调度（3 周）

核心目标：**更智能的调度和联动**

- Agent 依赖链（`--after`）
- 并发控制
- macOS 休眠唤醒补跑
- Webhook 触发（外部系统回调启动 Agent）
- 失败重试策略（固定间隔 / 指数退避）

### Phase 4：生态（持续）

- 社区 Agent 命令分享（类似 dotfiles 仓库）
- 多机管理（通过 SSH/API 管理远程 Agent）
- VS Code 扩展
- Linux 支持

---

## 七、与现有工具对比

| 特性 | AgentBox | PM2 | launchd | n8n | cron |
|------|----------|-----|---------|-----|------|
| 注册方式 | 一行命令 | 配置文件或命令 | 手写 plist | Web UI 拖拽 | crontab 编辑 |
| Web Dashboard | ✅ 内置免费 | ⚠️ 付费 | ❌ | ✅ | ❌ |
| 日志聚合 | ✅ | ✅ | ❌ 分散 | ✅ | ❌ |
| 故障告警 | ✅ 多渠道 | ⚠️ 基础 | ❌ | ✅ | ❌ |
| 依赖链 | ✅ | ❌ | ❌ | ✅ | ❌ |
| 休眠补跑 | ✅ | ❌ | ⚠️ | N/A | ❌ |
| 安装复杂度 | `curl \| sh` | `npm install` | 系统自带 | Docker | 系统自带 |
| 资源占用 | 极低 | 低 | 极低 | 中 | 极低 |

---

## 八、安装与快速开始

### 安装

一行命令安装：

```
curl -fsSL https://raw.githubusercontent.com/anthropic/agentbox/main/install.sh | sh
```

安装脚本会自动检测系统架构（Apple Silicon / Intel），从 GitHub Releases 下载对应的预编译二进制，放到 `~/.local/bin/` 并添加到 PATH。

也支持 Homebrew：`brew install agentbox`

也支持 Cargo：`cargo install agentbox`

安装后执行 `agentbox daemon start` 启动后台守护进程。

### 发布

项目托管在 GitHub，通过 GitHub Actions 自动构建和发布：

- 每次打 tag 自动触发 CI，编译 macOS (aarch64 + x86_64) 和 Linux (x86_64) 三个平台的二进制
- 自动上传到 GitHub Releases
- install.sh 脚本从 Releases 拉取最新版本

### 3 分钟上手

```
# 1. 注册一个 Agent
agentbox register morning-brief "claude -p '总结今日科技新闻要点，保存到 ~/briefs/'"

# 2. 设置每天早上8点执行
agentbox schedule morning-brief "0 8 * * *"

# 3. 手动跑一次看看效果
agentbox run morning-brief

# 4. 看日志
agentbox logs morning-brief

# 5. 再注册几个
agentbox register stock-scan "python ~/scripts/stock_scan.py"
agentbox schedule stock-scan "0 18 * * 1-5"

agentbox register repo-cleanup "cd ~/projects && bash cleanup.sh"
agentbox schedule repo-cleanup --every 7d

# 6. 看全局状态
agentbox list

# 7. 打开 Dashboard
agentbox dashboard
```

完事了。不需要建目录，不需要写配置文件，不需要懂 launchd plist 语法。

---

## 九、商业思考

### 开源策略

核心功能完全开源（MIT License），包括 CLI、调度引擎、日志、Dashboard。

可选付费方向：

- **AgentBox Cloud**：多设备同步 Dashboard、远程查看、历史数据云备份
- **团队版**：多人共享 Agent 配置和运行状态

### 增长路径

1. 开源获取用户 → 解决个人开发者的真实痛点
2. 社区建设 → Agent 命令分享仓库，像 dotfiles 一样互相参考
3. 企业版 → 团队管理、权限控制、审计日志

### 6 个月目标

- GitHub Stars > 2,000
- 月活用户 > 500

---

## 十、FAQ

**Q: 跟 cron 有什么区别？**  
cron 能调度但看不到状态、没有日志聚合、失败了不通知。AgentBox 在调度之上加了可观测性和运维能力。

**Q: 跟 PM2 有什么区别？**  
PM2 主要管长期运行的服务进程。AgentBox 面向的是"定时跑一次就退出"的 Agent 任务，并且注册方式更简单——一行命令而不是配置文件。

**Q: 需要 Docker 吗？**  
不需要。Rust 编译的单二进制，`curl | sh` 安装，直接管理本地进程。你的 Agent 命令里如果用到 Docker 也完全没问题。

**Q: 支持什么语言的 Agent？**  
任何语言。AgentBox 不关心命令里是 Python、Node、Shell、Go 还是 Claude CLI，只要是能在终端执行的命令就行。

**Q: 数据存在哪里？**  
所有数据存在本地 `~/.agentbox/` 目录，不发送任何数据到云端。

**Q: macOS 重启后 Agent 还会自动运行吗？**  
会。`agentbox daemon start` 会注册 macOS LaunchAgent，开机自动启动守护进程，调度照常执行。
