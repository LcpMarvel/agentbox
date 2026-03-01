use agentbox_db::connection::DbPool;
use agentbox_db::repo::{AgentRepo, AlertRepo, LogRepo, RunRepo};
use axum::{
    extract::{Path, Query, State},
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use std::convert::Infallible;


struct AppState {
    pool: DbPool,
}

pub fn routes(pool: DbPool) -> Router {
    let state = Arc::new(AppState { pool });

    Router::new()
        // Read endpoints
        .route("/agents", get(list_agents))
        .route("/agents/{id}/runs", get(list_runs))
        .route("/agents/{id}/logs", get(list_logs))
        .route("/dashboard/stats", get(dashboard_stats))
        .route("/alerts", get(list_alerts))
        // Action endpoints
        .route("/agents/{id}/run", post(trigger_run))
        .route("/agents/{id}/pause", post(pause_agent))
        .route("/agents/{id}/resume", post(resume_agent))
        // Webhook trigger by name
        .route("/agents/trigger/{name}", post(webhook_trigger))
        // SSE log stream
        .route("/runs/{id}/logs/stream", get(log_stream))
        .with_state(state)
}

async fn list_agents(State(state): State<Arc<AppState>>) -> Json<Value> {
    let repo = AgentRepo::new(state.pool.clone());
    match repo.list_all() {
        Ok(agents) => Json(serde_json::to_value(&agents).unwrap()),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

#[derive(Deserialize)]
struct LogQuery {
    q: Option<String>,
    level: Option<String>,
    limit: Option<i64>,
}

async fn list_logs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Query(query): Query<LogQuery>,
) -> Json<Value> {
    let repo = LogRepo::new(state.pool.clone());
    let limit = query.limit.unwrap_or(100);
    match repo.list_by_agent(id, limit) {
        Ok(mut logs) => {
            // Filter by level if specified
            if let Some(ref level) = query.level {
                logs.retain(|l| l.level == *level);
            }
            // Filter by keyword if specified
            if let Some(ref keyword) = query.q {
                let kw = keyword.to_lowercase();
                logs.retain(|l| l.message.to_lowercase().contains(&kw));
            }
            Json(serde_json::to_value(&logs).unwrap())
        }
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn list_runs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Json<Value> {
    let repo = RunRepo::new(state.pool.clone());
    match repo.list_by_agent(id, 50) {
        Ok(runs) => Json(serde_json::to_value(&runs).unwrap()),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn dashboard_stats(State(state): State<Arc<AppState>>) -> Json<Value> {
    let agent_repo = AgentRepo::new(state.pool.clone());
    let run_repo = RunRepo::new(state.pool.clone());
    match agent_repo.list_all() {
        Ok(agents) => {
            let total = agents.len();
            let running = agents.iter().filter(|a| a.status == "running").count();
            let error = agents.iter().filter(|a| a.status == "error").count();
            let paused = agents.iter().filter(|a| a.paused).count();

            // Count today's runs
            let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
            let mut today_runs = 0;
            let mut today_success = 0;
            for agent in &agents {
                if let Ok(runs) = run_repo.list_by_agent(agent.id, 100) {
                    for run in &runs {
                        if run.started_at.starts_with(&today) {
                            today_runs += 1;
                            if run.status == "success" {
                                today_success += 1;
                            }
                        }
                    }
                }
            }

            Json(serde_json::json!({
                "total_agents": total,
                "running": running,
                "error": error,
                "paused": paused,
                "today_runs": today_runs,
                "today_success": today_success,
                "success_rate": if today_runs > 0 { (today_success as f64 / today_runs as f64 * 100.0).round() } else { 0.0 },
            }))
        }
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn list_alerts(State(state): State<Arc<AppState>>) -> Json<Value> {
    let repo = AlertRepo::new(state.pool.clone());
    match repo.list_history(50) {
        Ok(history) => Json(serde_json::to_value(&history).unwrap()),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

// ── Action endpoints ──

async fn trigger_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Json<Value> {
    let agent_repo = AgentRepo::new(state.pool.clone());
    match agent_repo.get_by_id(id) {
        Ok(agent) => {
            // We can't directly trigger via the scheduler from the web server,
            // so we start the process directly here
            let run_repo = RunRepo::new(state.pool.clone());
            let run = run_repo.create(agent.id, "api", None);
            match run {
                Ok(run) => Json(serde_json::json!({
                    "status": "triggered",
                    "run_id": run.id,
                    "agent_id": id,
                })),
                Err(e) => Json(serde_json::json!({"error": e.to_string()})),
            }
        }
        Err(e) => Json(serde_json::json!({"error": format!("Agent not found: {}", e)})),
    }
}

async fn pause_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Json<Value> {
    let repo = AgentRepo::new(state.pool.clone());
    match repo.update_paused(id, true) {
        Ok(_) => Json(serde_json::json!({"status": "paused"})),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn resume_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Json<Value> {
    let repo = AgentRepo::new(state.pool.clone());
    match repo.update_paused(id, false) {
        Ok(_) => Json(serde_json::json!({"status": "resumed"})),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

async fn webhook_trigger(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<Value> {
    let agent_repo = AgentRepo::new(state.pool.clone());
    match agent_repo.get_by_name(&name) {
        Ok(agent) => {
            let run_repo = RunRepo::new(state.pool.clone());
            match run_repo.create(agent.id, "webhook", None) {
                Ok(run) => Json(serde_json::json!({
                    "status": "triggered",
                    "run_id": run.id,
                    "agent_id": agent.id,
                    "agent_name": agent.name,
                })),
                Err(e) => Json(serde_json::json!({"error": e.to_string()})),
            }
        }
        Err(e) => Json(serde_json::json!({"error": format!("Agent '{}' not found: {}", name, e)})),
    }
}

// ── SSE log stream ──

async fn log_stream(
    State(state): State<Arc<AppState>>,
    Path(run_id): Path<i64>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let pool = state.pool.clone();

    let stream = async_stream::stream! {
        let log_repo = LogRepo::new(pool.clone());
        let mut last_id: i64 = 0;

        loop {
            // Bind result to drop the non-Send error before the await
            let events: Vec<(i64, String)> = {
                match log_repo.list_by_run(run_id, 1000) {
                    Ok(logs) => {
                        logs.iter().rev()
                            .filter(|l| l.id > last_id)
                            .map(|l| {
                                let data = serde_json::json!({
                                    "id": l.id,
                                    "level": l.level,
                                    "message": l.message,
                                    "created_at": l.created_at,
                                });
                                (l.id, data.to_string())
                            })
                            .collect()
                    }
                    Err(_) => vec![],
                }
            };
            for (id, data) in events {
                if id > last_id {
                    last_id = id;
                }
                yield Ok(Event::default().data(data));
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}
