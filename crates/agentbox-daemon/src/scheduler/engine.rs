use crate::alert::AlertManager;
use agentbox_db::repo::{AgentRepo, ConfigRepo, LogRepo, RunRepo};
use chrono::{DateTime, Utc};
use cron::Schedule;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::str::FromStr;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub enum SchedulerEvent {
    Reload,
    RunNow {
        agent_id: i64,
        trigger: String,
    },
    Pause {
        agent_id: i64,
    },
    Resume {
        agent_id: i64,
    },
    /// An agent completed (success or failure) — check dependency chain
    AgentCompleted {
        agent_id: i64,
        success: bool,
    },
    Shutdown,
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ScheduledJob {
    agent_id: i64,
    next_run: DateTime<Utc>,
}

impl Ord for ScheduledJob {
    fn cmp(&self, other: &Self) -> Ordering {
        other.next_run.cmp(&self.next_run)
    }
}

impl PartialOrd for ScheduledJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct SchedulerEngine {
    agent_repo: AgentRepo,
    run_repo: RunRepo,
    log_repo: LogRepo,
    config_repo: ConfigRepo,
    alert_manager: AlertManager,
    event_tx: mpsc::Sender<SchedulerEvent>,
}

impl SchedulerEngine {
    pub fn new(
        agent_repo: AgentRepo,
        run_repo: RunRepo,
        log_repo: LogRepo,
        config_repo: ConfigRepo,
        alert_manager: AlertManager,
        event_tx: mpsc::Sender<SchedulerEvent>,
    ) -> Self {
        Self {
            agent_repo,
            run_repo,
            log_repo,
            config_repo,
            alert_manager,
            event_tx,
        }
    }

    fn max_concurrent(&self) -> usize {
        self.config_repo
            .get("max_concurrent")
            .ok()
            .flatten()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(10)
    }

    fn running_count(&self) -> usize {
        self.agent_repo
            .list_all()
            .map(|agents| agents.iter().filter(|a| a.status == "running").count())
            .unwrap_or(0)
    }

    pub async fn run(self, mut event_rx: mpsc::Receiver<SchedulerEvent>) {
        info!("Scheduler engine started");

        let mut heap: BinaryHeap<ScheduledJob> = BinaryHeap::new();
        self.load_jobs(&mut heap);
        self.catchup_missed_runs(&mut heap).await;

        loop {
            let sleep_duration = if let Some(next) = heap.peek() {
                let now = Utc::now();
                if next.next_run <= now {
                    std::time::Duration::from_millis(0)
                } else {
                    (next.next_run - now)
                        .to_std()
                        .unwrap_or(std::time::Duration::from_secs(60))
                }
            } else {
                std::time::Duration::from_secs(60)
            };

            tokio::select! {
                _ = tokio::time::sleep(sleep_duration) => {
                    while let Some(job) = heap.peek() {
                        if job.next_run <= Utc::now() {
                            let job = heap.pop().unwrap();

                            // Concurrency control
                            if self.running_count() >= self.max_concurrent() {
                                warn!("Max concurrent limit reached, re-queuing agent {}", job.agent_id);
                                heap.push(ScheduledJob {
                                    agent_id: job.agent_id,
                                    next_run: Utc::now() + chrono::Duration::seconds(10),
                                });
                                break;
                            }

                            self.fire_job(job.agent_id).await;
                            if let Some(next) = self.compute_next_run(job.agent_id) {
                                heap.push(ScheduledJob {
                                    agent_id: job.agent_id,
                                    next_run: next,
                                });
                            }
                        } else {
                            break;
                        }
                    }
                }
                event = event_rx.recv() => {
                    match event {
                        Some(SchedulerEvent::Reload) => {
                            heap.clear();
                            self.load_jobs(&mut heap);
                        }
                        Some(SchedulerEvent::RunNow { agent_id, trigger }) => {
                            if self.running_count() >= self.max_concurrent() {
                                warn!("Max concurrent limit reached, cannot run agent {} now", agent_id);
                            } else {
                                self.execute_agent(agent_id, &trigger).await;
                            }
                        }
                        Some(SchedulerEvent::Pause { agent_id }) => {
                            if let Err(e) = self.agent_repo.update_paused(agent_id, true) {
                                error!("Failed to pause agent {}: {}", agent_id, e);
                            }
                            heap.retain(|j| j.agent_id != agent_id);
                        }
                        Some(SchedulerEvent::Resume { agent_id }) => {
                            if let Err(e) = self.agent_repo.update_paused(agent_id, false) {
                                error!("Failed to resume agent {}: {}", agent_id, e);
                            }
                            if let Some(next) = self.compute_next_run(agent_id) {
                                heap.push(ScheduledJob { agent_id, next_run: next });
                            }
                        }
                        Some(SchedulerEvent::AgentCompleted { agent_id, success }) => {
                            if success {
                                self.trigger_dependents(agent_id).await;
                            }
                        }
                        Some(SchedulerEvent::Shutdown) | None => {
                            info!("Scheduler shutting down");
                            break;
                        }
                    }
                }
            }
        }
    }

    fn load_jobs(&self, heap: &mut BinaryHeap<ScheduledJob>) {
        match self.agent_repo.list_scheduled() {
            Ok(agents) => {
                for agent in agents {
                    if let Some(next) = self.compute_next_run(agent.id) {
                        heap.push(ScheduledJob {
                            agent_id: agent.id,
                            next_run: next,
                        });
                    }
                }
                info!("Loaded {} scheduled jobs", heap.len());
            }
            Err(e) => error!("Failed to load scheduled agents: {}", e),
        }
    }

    /// On daemon start, check for missed schedules and run them immediately.
    async fn catchup_missed_runs(&self, heap: &mut BinaryHeap<ScheduledJob>) {
        let mut to_run = Vec::new();
        let now = Utc::now();

        let agents = match self.agent_repo.list_scheduled() {
            Ok(a) => a,
            Err(e) => {
                error!("Failed to check missed runs: {}", e);
                return;
            }
        };

        for agent in agents {
            if agent.schedule_type == "after" {
                continue;
            }
            if let Some(ref last_run) = agent.last_run_at {
                if let Ok(last) = DateTime::parse_from_rfc3339(last_run) {
                    let last_utc = last.with_timezone(&Utc);
                    if let Some(expected_next) = self.compute_next_run_from(
                        &agent.schedule_type,
                        agent.cron_expr.as_deref(),
                        agent.interval_secs,
                        &last_utc,
                    ) {
                        if expected_next < now {
                            info!("Catching up missed run for agent '{}'", agent.name);
                            to_run.push(agent.id);
                        }
                    }
                }
            }
        }

        for agent_id in to_run {
            self.execute_agent(agent_id, "catchup").await;
            heap.retain(|j| j.agent_id != agent_id);
            if let Some(next) = self.compute_next_run(agent_id) {
                heap.push(ScheduledJob {
                    agent_id,
                    next_run: next,
                });
            }
        }
    }

    fn compute_next_run_from(
        &self,
        schedule_type: &str,
        cron_expr: Option<&str>,
        interval_secs: Option<i64>,
        base: &DateTime<Utc>,
    ) -> Option<DateTime<Utc>> {
        match schedule_type {
            "cron" => {
                let expr = cron_expr?;
                let schedule = Schedule::from_str(expr).ok()?;
                schedule.after(base).next()
            }
            "interval" => {
                let secs = interval_secs?;
                Some(*base + chrono::Duration::seconds(secs))
            }
            _ => None,
        }
    }

    fn compute_next_run(&self, agent_id: i64) -> Option<DateTime<Utc>> {
        let agent = self.agent_repo.get_by_id(agent_id).ok()?;
        if agent.paused {
            return None;
        }

        match agent.schedule_type.as_str() {
            "cron" => {
                let expr = agent.cron_expr.as_ref()?;
                let schedule = Schedule::from_str(expr).ok()?;
                schedule.upcoming(Utc).next()
            }
            "interval" => {
                let secs = agent.interval_secs?;
                let base = if let Some(ref last) = agent.last_run_at {
                    DateTime::parse_from_rfc3339(last)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now())
                } else {
                    Utc::now()
                };
                Some(base + chrono::Duration::seconds(secs))
            }
            _ => None, // "after" and "manual" don't have time-based scheduling
        }
    }

    /// Trigger agents that depend on the completed agent.
    async fn trigger_dependents(&self, completed_agent_id: i64) {
        let dependents = match self.agent_repo.list_dependents(completed_agent_id) {
            Ok(deps) => deps,
            Err(e) => {
                error!(
                    "Failed to find dependents of agent {}: {}",
                    completed_agent_id, e
                );
                return;
            }
        };
        for dep in dependents {
            info!(
                "Triggering dependent agent '{}' (after agent {})",
                dep.name, completed_agent_id
            );
            self.execute_agent(dep.id, "after").await;
        }
    }

    async fn fire_job(&self, agent_id: i64) {
        let trigger = {
            match self.agent_repo.get_by_id(agent_id) {
                Ok(agent) => agent.schedule_type.clone(),
                Err(_) => return,
            }
        };
        self.execute_agent(agent_id, &trigger).await;
    }

    async fn execute_agent(&self, agent_id: i64, trigger: &str) {
        let agent = match self.agent_repo.get_by_id(agent_id) {
            Ok(a) => a,
            Err(e) => {
                error!("Agent {} not found: {}", agent_id, e);
                return;
            }
        };

        info!("Executing agent '{}' (trigger={})", agent.name, trigger);

        let result = crate::executor::run_agent(
            &agent,
            trigger,
            &self.run_repo,
            &self.log_repo,
            &self.agent_repo,
            Some(&self.alert_manager),
        )
        .await;

        let success = result.is_ok();

        match result {
            Ok(_) => info!("Agent '{}' completed", agent.name),
            Err(e) => error!("Agent '{}' failed: {}", agent.name, e),
        }

        // Notify for dependency chain
        let _ = self
            .event_tx
            .send(SchedulerEvent::AgentCompleted { agent_id, success })
            .await;
    }
}
