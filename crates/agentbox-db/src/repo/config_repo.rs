use crate::connection::DbPool;
use rusqlite::params;

pub struct ConfigRepo {
    pool: DbPool,
}

impl ConfigRepo {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub fn get(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        let result = conn.query_row(
            "SELECT value FROM config WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        conn.execute("DELETE FROM config WHERE key = ?1", params![key])?;
        Ok(())
    }

    pub fn list_all(&self) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT key, value FROM config ORDER BY key")?;
        let items = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(items)
    }
}
