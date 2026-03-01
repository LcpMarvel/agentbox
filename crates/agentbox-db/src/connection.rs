use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::path::Path;

use crate::migrations;

pub type DbPool = Pool<SqliteConnectionManager>;

/// Create a connection pool for the given SQLite database path.
pub fn create_pool(db_path: &Path) -> Result<DbPool, Box<dyn std::error::Error>> {
    let manager = SqliteConnectionManager::file(db_path);
    let pool = Pool::builder().max_size(8).build(manager)?;

    // Configure SQLite pragmas
    {
        let conn = pool.get()?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA synchronous = NORMAL;
             PRAGMA busy_timeout = 5000;",
        )?;
    }

    // Run migrations
    migrations::run_migrations(&pool)?;

    Ok(pool)
}

/// Create an in-memory pool for testing.
pub fn create_memory_pool() -> Result<DbPool, Box<dyn std::error::Error>> {
    let manager = SqliteConnectionManager::memory();
    let pool = Pool::builder().max_size(1).build(manager)?;

    {
        let conn = pool.get()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    }

    migrations::run_migrations(&pool)?;

    Ok(pool)
}
