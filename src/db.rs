use std::path::PathBuf;

use anyhow::{Context, Result};
use rusqlite::Connection;

/// v1 — current schema. Completed tasks stay in `tasks` with status='done'
/// and a `done_at` timestamp; there is no separate `done` table.
const MIGRATION_V1: &str = r#"
CREATE TABLE IF NOT EXISTS plans (
    id           INTEGER PRIMARY KEY,
    title        TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    status       TEXT NOT NULL DEFAULT 'active'
                 CHECK (status IN ('active', 'done', 'archived')),
    priority     INTEGER NOT NULL DEFAULT 1
                 CHECK (priority BETWEEN 0 AND 3),
    due_date     TEXT,
    start_date   TEXT,
    created_at   TEXT NOT NULL DEFAULT (datetime('now','localtime')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE IF NOT EXISTS tasks (
    id           INTEGER PRIMARY KEY,
    adr_id       INTEGER REFERENCES adrs(id) ON DELETE SET NULL,
    plan_id      INTEGER REFERENCES plans(id) ON DELETE SET NULL,
    type         TEXT NOT NULL DEFAULT 'feature'
                 CHECK (type IN ('feature', 'bug', 'chore', 'refactor', 'docs')),
    title        TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    status       TEXT NOT NULL DEFAULT 'todo'
                 CHECK (status IN ('todo', 'active', 'blocked', 'done')),
    resolution   TEXT
                 CHECK (resolution IS NULL OR resolution IN ('done', 'abandoned', 'duplicate')),
    priority     INTEGER NOT NULL DEFAULT 1
                 CHECK (priority BETWEEN 0 AND 3),
    tags         TEXT NOT NULL DEFAULT '',
    time_spent   REAL NOT NULL DEFAULT 0,
    due_date     TEXT,
    done_at      TEXT,
    started_at   TEXT,
    created_at   TEXT NOT NULL DEFAULT (datetime('now','localtime')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE IF NOT EXISTS adrs (
    id            INTEGER PRIMARY KEY,
    number        TEXT NOT NULL UNIQUE,
    title         TEXT NOT NULL,
    context       TEXT NOT NULL,
    decision      TEXT NOT NULL,
    consequence   TEXT NOT NULL DEFAULT '',
    status        TEXT NOT NULL DEFAULT 'proposed'
                  CHECK (status IN ('proposed', 'accepted', 'superseded')),
    superseded_by INTEGER REFERENCES adrs(id),
    decided_at    TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);
-- 索引同你的设计,外加:
CREATE INDEX IF NOT EXISTS idx_tasks_resolution ON tasks(resolution);
CREATE INDEX IF NOT EXISTS idx_tasks_plan ON tasks(plan_id);
CREATE INDEX IF NOT EXISTS idx_tasks_adr ON tasks(adr_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status_priority ON tasks(status, priority DESC);
CREATE INDEX IF NOT EXISTS idx_tasks_due ON tasks(due_date);
CREATE INDEX IF NOT EXISTS idx_tasks_done_at ON tasks(done_at);
"#;

const MIGRATIONS: &[&str] = &[MIGRATION_V1];

fn db_path() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let mut dir: Option<&std::path::Path> = Some(&cwd);
    while let Some(d) = dir {
        let candidate = d.join(".dp").join("dp.db");
        if candidate.exists() {
            return Ok(candidate);
        }
        dir = d.parent();
    }
    Ok(cwd.join(".dp").join("dp.db"))
}

pub fn init() -> Result<()> {
    let path = db_path()?;
    if path.exists() {
        println!("dp database already exists at {}", path.display());
        return Ok(());
    }
    std::fs::create_dir_all(path.parent().expect("db path has a parent"))?;
    let conn =
        Connection::open(&path).with_context(|| format!("failed to open {}", path.display()))?;
    configure(&conn)?;
    migrate(&conn)?;
    println!("Initialized dp database at {}", path.display());
    Ok(())
}

pub fn open() -> Result<Connection> {
    let path = db_path()?;
    if !path.exists() {
        anyhow::bail!(
            "dp database not found at {}\nRun `dp init` in your project root first.",
            path.display()
        );
    }
    let conn =
        Connection::open(&path).with_context(|| format!("failed to open {}", path.display()))?;
    configure(&conn)?;
    migrate(&conn)?; // auto-upgrades old databases on first run
    Ok(conn)
}

fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?; // OFF by default in rusqlite!
    conn.pragma_update(None, "busy_timeout", 5000)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    Ok(())
}

/// Sequential, versioned migrations tracked via PRAGMA user_version.
/// Each migration runs inside its own transaction.
pub fn migrate(conn: &Connection) -> Result<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate() {
        let target = (i + 1) as i64;
        if current >= target {
            continue;
        }
        conn.execute_batch(&format!(
            "BEGIN IMMEDIATE;\n{sql}\nPRAGMA user_version = {target};\nCOMMIT;"
        ))
        .with_context(|| format!("applying schema migration v{target}"))?;
    }
    Ok(())
}
