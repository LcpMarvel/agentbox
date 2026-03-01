use crate::connection::DbPool;
use crate::models::Agent;
use rusqlite::params;

const AGENT_SELECT: &str = "SELECT id, name, command, working_dir, env_vars, schedule_type, cron_expr,
        interval_secs, after_agent_id, status, paused, timeout_secs, max_retries,
        created_at, last_run_at, next_run_at, retry_delay_secs, retry_strategy
 FROM agents";

fn row_to_agent(row: &rusqlite::Row) -> rusqlite::Result<Agent> {
    Ok(Agent {
        id: row.get(0)?,
        name: row.get(1)?,
        command: row.get(2)?,
        working_dir: row.get(3)?,
        env_vars: row.get(4)?,
        schedule_type: row.get(5)?,
        cron_expr: row.get(6)?,
        interval_secs: row.get(7)?,
        after_agent_id: row.get(8)?,
        status: row.get(9)?,
        paused: row.get(10)?,
        timeout_secs: row.get(11)?,
        max_retries: row.get(12)?,
        created_at: row.get(13)?,
        last_run_at: row.get(14)?,
        next_run_at: row.get(15)?,
        retry_delay_secs: row.get(16)?,
        retry_strategy: row.get(17)?,
    })
}

pub struct AgentRepo {
    pool: DbPool,
}

impl AgentRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn create(
        &self,
        name: &str,
        command: &str,
        working_dir: Option<&str>,
        env_vars: Option<&str>,
    ) -> Result<Agent, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO agents (name, command, working_dir, env_vars) VALUES (?1, ?2, ?3, ?4)",
            params![name, command, working_dir, env_vars.unwrap_or("{}")],
        )?;
        let id = conn.last_insert_rowid();
        self.get_by_id(id)
    }

    pub fn get_by_id(&self, id: i64) -> Result<Agent, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        let sql = format!("{} WHERE id = ?1", AGENT_SELECT);
        let agent = conn.query_row(&sql, params![id], row_to_agent)?;
        Ok(agent)
    }

    pub fn get_by_name(&self, name: &str) -> Result<Agent, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        let sql = format!("{} WHERE name = ?1", AGENT_SELECT);
        let agent = conn.query_row(&sql, params![name], row_to_agent)?;
        Ok(agent)
    }

    pub fn list_all(&self) -> Result<Vec<Agent>, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        let sql = format!("{} ORDER BY id", AGENT_SELECT);
        let mut stmt = conn.prepare(&sql)?;
        let agents = stmt
            .query_map([], row_to_agent)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(agents)
    }

    pub fn list_scheduled(&self) -> Result<Vec<Agent>, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        let sql = format!("{} WHERE schedule_type != 'manual' AND paused = 0 ORDER BY next_run_at", AGENT_SELECT);
        let mut stmt = conn.prepare(&sql)?;
        let agents = stmt
            .query_map([], row_to_agent)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(agents)
    }

    pub fn list_dependents(&self, after_agent_id: i64) -> Result<Vec<Agent>, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        let sql = format!("{} WHERE schedule_type = 'after' AND after_agent_id = ?1 AND paused = 0", AGENT_SELECT);
        let mut stmt = conn.prepare(&sql)?;
        let agents = stmt
            .query_map(params![after_agent_id], row_to_agent)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(agents)
    }

    pub fn update_schedule(
        &self,
        id: i64,
        schedule_type: &str,
        cron_expr: Option<&str>,
        interval_secs: Option<i64>,
        after_agent_id: Option<i64>,
        next_run_at: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE agents SET schedule_type = ?1, cron_expr = ?2, interval_secs = ?3,
             after_agent_id = ?4, next_run_at = ?5 WHERE id = ?6",
            params![schedule_type, cron_expr, interval_secs, after_agent_id, next_run_at, id],
        )?;
        Ok(())
    }

    pub fn update_status(&self, id: i64, status: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE agents SET status = ?1 WHERE id = ?2",
            params![status, id],
        )?;
        Ok(())
    }

    pub fn update_paused(&self, id: i64, paused: bool) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE agents SET paused = ?1 WHERE id = ?2",
            params![paused, id],
        )?;
        Ok(())
    }

    pub fn update_last_run(&self, id: i64, last_run_at: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE agents SET last_run_at = ?1 WHERE id = ?2",
            params![last_run_at, id],
        )?;
        Ok(())
    }

    pub fn update_next_run(&self, id: i64, next_run_at: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE agents SET next_run_at = ?1 WHERE id = ?2",
            params![next_run_at, id],
        )?;
        Ok(())
    }

    pub fn update_retry_config(
        &self,
        id: i64,
        max_retries: i64,
        retry_delay_secs: i64,
        retry_strategy: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE agents SET max_retries = ?1, retry_delay_secs = ?2, retry_strategy = ?3 WHERE id = ?4",
            params![max_retries, retry_delay_secs, retry_strategy, id],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: i64) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        conn.execute("DELETE FROM agents WHERE id = ?1", params![id])?;
        Ok(())
    }
}
