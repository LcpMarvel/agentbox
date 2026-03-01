use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

pub fn run_migrations(pool: &Pool<SqliteConnectionManager>) -> Result<(), Box<dyn std::error::Error>> {
    let conn = pool.get()?;

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY
        );",
    )?;

    let current: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if current < 1 {
        m001_initial(&conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (?1)", [1])?;
    }

    if current < 2 {
        m002_alerts_and_retry(&conn)?;
        conn.execute("INSERT INTO schema_version (version) VALUES (?1)", [2])?;
    }

    Ok(())
}

fn m001_initial(conn: &rusqlite::Connection) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS agents (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            name          TEXT NOT NULL UNIQUE,
            command       TEXT NOT NULL,
            working_dir   TEXT,
            env_vars      TEXT NOT NULL DEFAULT '{}',
            schedule_type TEXT NOT NULL DEFAULT 'manual',
            cron_expr     TEXT,
            interval_secs INTEGER,
            after_agent_id INTEGER REFERENCES agents(id),
            status        TEXT NOT NULL DEFAULT 'idle',
            paused        INTEGER NOT NULL DEFAULT 0,
            timeout_secs  INTEGER,
            max_retries   INTEGER NOT NULL DEFAULT 0,
            created_at    TEXT NOT NULL DEFAULT (datetime('now')),
            last_run_at   TEXT,
            next_run_at   TEXT
        );

        CREATE TABLE IF NOT EXISTS runs (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_id     INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
            status       TEXT NOT NULL DEFAULT 'running',
            trigger_type TEXT NOT NULL,
            started_at   TEXT NOT NULL DEFAULT (datetime('now')),
            ended_at     TEXT,
            duration_ms  INTEGER,
            exit_code    INTEGER,
            error_message TEXT,
            pid          INTEGER,
            retry_count  INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS logs (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_id   INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
            run_id     INTEGER NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
            level      TEXT NOT NULL DEFAULT 'stdout',
            message    TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_runs_agent_id ON runs(agent_id);
        CREATE INDEX IF NOT EXISTS idx_logs_run_id ON logs(run_id);
        CREATE INDEX IF NOT EXISTS idx_logs_agent_id ON logs(agent_id);",
    )?;
    Ok(())
}

fn m002_alerts_and_retry(conn: &rusqlite::Connection) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute_batch(
        "-- Alert configuration table
        CREATE TABLE IF NOT EXISTS alert_channels (
            id       INTEGER PRIMARY KEY AUTOINCREMENT,
            channel  TEXT NOT NULL,
            config   TEXT NOT NULL DEFAULT '{}',
            enabled  INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Alert history
        CREATE TABLE IF NOT EXISTS alert_history (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_id   INTEGER NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
            run_id     INTEGER REFERENCES runs(id) ON DELETE CASCADE,
            alert_type TEXT NOT NULL,
            channel    TEXT NOT NULL,
            message    TEXT NOT NULL,
            sent_at    TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- Add retry config columns to agents
        ALTER TABLE agents ADD COLUMN retry_delay_secs INTEGER NOT NULL DEFAULT 30;
        ALTER TABLE agents ADD COLUMN retry_strategy TEXT NOT NULL DEFAULT 'fixed';

        -- Global config table
        CREATE TABLE IF NOT EXISTS config (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_alert_history_agent ON alert_history(agent_id);",
    )?;
    Ok(())
}
