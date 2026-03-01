use crate::connection::DbPool;
use crate::models::Run;
use rusqlite::params;

pub struct RunRepo {
    pool: DbPool,
}

impl RunRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn create(
        &self,
        agent_id: i64,
        trigger_type: &str,
        pid: Option<i64>,
    ) -> Result<Run, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO runs (agent_id, trigger_type, pid) VALUES (?1, ?2, ?3)",
            params![agent_id, trigger_type, pid],
        )?;
        let id = conn.last_insert_rowid();
        self.get_by_id(id)
    }

    pub fn get_by_id(&self, id: i64) -> Result<Run, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        let run = conn.query_row(
            "SELECT id, agent_id, status, trigger_type, started_at, ended_at,
                    duration_ms, exit_code, error_message, pid, retry_count
             FROM runs WHERE id = ?1",
            params![id],
            |row| {
                Ok(Run {
                    id: row.get(0)?,
                    agent_id: row.get(1)?,
                    status: row.get(2)?,
                    trigger_type: row.get(3)?,
                    started_at: row.get(4)?,
                    ended_at: row.get(5)?,
                    duration_ms: row.get(6)?,
                    exit_code: row.get(7)?,
                    error_message: row.get(8)?,
                    pid: row.get(9)?,
                    retry_count: row.get(10)?,
                })
            },
        )?;
        Ok(run)
    }

    pub fn list_by_agent(
        &self,
        agent_id: i64,
        limit: i64,
    ) -> Result<Vec<Run>, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, status, trigger_type, started_at, ended_at,
                    duration_ms, exit_code, error_message, pid, retry_count
             FROM runs WHERE agent_id = ?1 ORDER BY id DESC LIMIT ?2",
        )?;
        let runs = stmt
            .query_map(params![agent_id, limit], |row| {
                Ok(Run {
                    id: row.get(0)?,
                    agent_id: row.get(1)?,
                    status: row.get(2)?,
                    trigger_type: row.get(3)?,
                    started_at: row.get(4)?,
                    ended_at: row.get(5)?,
                    duration_ms: row.get(6)?,
                    exit_code: row.get(7)?,
                    error_message: row.get(8)?,
                    pid: row.get(9)?,
                    retry_count: row.get(10)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(runs)
    }

    pub fn finish(
        &self,
        id: i64,
        status: &str,
        exit_code: Option<i32>,
        error_message: Option<&str>,
        duration_ms: i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        conn.execute(
            "UPDATE runs SET status = ?1, exit_code = ?2, error_message = ?3,
             duration_ms = ?4, ended_at = datetime('now') WHERE id = ?5",
            params![status, exit_code, error_message, duration_ms, id],
        )?;
        Ok(())
    }

    pub fn get_running_by_agent(&self, agent_id: i64) -> Result<Option<Run>, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        let result = conn.query_row(
            "SELECT id, agent_id, status, trigger_type, started_at, ended_at,
                    duration_ms, exit_code, error_message, pid, retry_count
             FROM runs WHERE agent_id = ?1 AND status = 'running' ORDER BY id DESC LIMIT 1",
            params![agent_id],
            |row| {
                Ok(Run {
                    id: row.get(0)?,
                    agent_id: row.get(1)?,
                    status: row.get(2)?,
                    trigger_type: row.get(3)?,
                    started_at: row.get(4)?,
                    ended_at: row.get(5)?,
                    duration_ms: row.get(6)?,
                    exit_code: row.get(7)?,
                    error_message: row.get(8)?,
                    pid: row.get(9)?,
                    retry_count: row.get(10)?,
                })
            },
        );
        match result {
            Ok(run) => Ok(Some(run)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
