use crate::connection::DbPool;
use crate::models::LogEntry;
use rusqlite::params;

pub struct LogRepo {
    pool: DbPool,
}

impl LogRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn insert(
        &self,
        agent_id: i64,
        run_id: i64,
        level: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO logs (agent_id, run_id, level, message) VALUES (?1, ?2, ?3, ?4)",
            params![agent_id, run_id, level, message],
        )?;
        Ok(())
    }

    pub fn list_by_agent(
        &self,
        agent_id: i64,
        limit: i64,
    ) -> Result<Vec<LogEntry>, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, run_id, level, message, created_at
             FROM logs WHERE agent_id = ?1 ORDER BY id DESC LIMIT ?2",
        )?;
        let logs = stmt
            .query_map(params![agent_id, limit], |row| {
                Ok(LogEntry {
                    id: row.get(0)?,
                    agent_id: row.get(1)?,
                    run_id: row.get(2)?,
                    level: row.get(3)?,
                    message: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(logs)
    }

    pub fn list_by_run(
        &self,
        run_id: i64,
        limit: i64,
    ) -> Result<Vec<LogEntry>, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, run_id, level, message, created_at
             FROM logs WHERE run_id = ?1 ORDER BY id DESC LIMIT ?2",
        )?;
        let logs = stmt
            .query_map(params![run_id, limit], |row| {
                Ok(LogEntry {
                    id: row.get(0)?,
                    agent_id: row.get(1)?,
                    run_id: row.get(2)?,
                    level: row.get(3)?,
                    message: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(logs)
    }

    pub fn list_all_recent(&self, limit: i64) -> Result<Vec<LogEntry>, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, run_id, level, message, created_at
             FROM logs ORDER BY id DESC LIMIT ?1",
        )?;
        let logs = stmt
            .query_map(params![limit], |row| {
                Ok(LogEntry {
                    id: row.get(0)?,
                    agent_id: row.get(1)?,
                    run_id: row.get(2)?,
                    level: row.get(3)?,
                    message: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(logs)
    }
}
