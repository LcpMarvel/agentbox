use crate::connection::DbPool;
use crate::models::{AlertChannel, AlertHistory};
use rusqlite::params;

pub struct AlertRepo {
    pool: DbPool,
}

impl AlertRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn add_channel(
        &self,
        channel: &str,
        config: &str,
    ) -> Result<AlertChannel, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO alert_channels (channel, config) VALUES (?1, ?2)",
            params![channel, config],
        )?;
        let id = conn.last_insert_rowid();
        let ch = conn.query_row(
            "SELECT id, channel, config, enabled, created_at FROM alert_channels WHERE id = ?1",
            params![id],
            |row| Ok(AlertChannel {
                id: row.get(0)?,
                channel: row.get(1)?,
                config: row.get(2)?,
                enabled: row.get(3)?,
                created_at: row.get(4)?,
            }),
        )?;
        Ok(ch)
    }

    pub fn list_enabled(&self) -> Result<Vec<AlertChannel>, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, channel, config, enabled, created_at FROM alert_channels WHERE enabled = 1",
        )?;
        let channels = stmt
            .query_map([], |row| Ok(AlertChannel {
                id: row.get(0)?,
                channel: row.get(1)?,
                config: row.get(2)?,
                enabled: row.get(3)?,
                created_at: row.get(4)?,
            }))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(channels)
    }

    pub fn list_all(&self) -> Result<Vec<AlertChannel>, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, channel, config, enabled, created_at FROM alert_channels ORDER BY id",
        )?;
        let channels = stmt
            .query_map([], |row| Ok(AlertChannel {
                id: row.get(0)?,
                channel: row.get(1)?,
                config: row.get(2)?,
                enabled: row.get(3)?,
                created_at: row.get(4)?,
            }))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(channels)
    }

    pub fn remove_channel(&self, id: i64) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        conn.execute("DELETE FROM alert_channels WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn record_alert(
        &self,
        agent_id: i64,
        run_id: Option<i64>,
        alert_type: &str,
        channel: &str,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT INTO alert_history (agent_id, run_id, alert_type, channel, message) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![agent_id, run_id, alert_type, channel, message],
        )?;
        Ok(())
    }

    pub fn list_history(
        &self,
        limit: i64,
    ) -> Result<Vec<AlertHistory>, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, agent_id, run_id, alert_type, channel, message, sent_at
             FROM alert_history ORDER BY id DESC LIMIT ?1",
        )?;
        let history = stmt
            .query_map(params![limit], |row| Ok(AlertHistory {
                id: row.get(0)?,
                agent_id: row.get(1)?,
                run_id: row.get(2)?,
                alert_type: row.get(3)?,
                channel: row.get(4)?,
                message: row.get(5)?,
                sent_at: row.get(6)?,
            }))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(history)
    }
}
