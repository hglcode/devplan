use std::path::PathBuf;

use anyhow::{Context, Result};
use rusqlite::Connection;

/// v1 — current schema. Completed tasks stay in `tasks` with status='done'
/// and a `done_at` timestamp; there is no separate `done` table.
const MIGRATION_V1: &str = include_str!("../sql/v1.sql");
const MIGRATION_V2: &str = include_str!("../sql/v2.sql");
const MIGRATIONS: &[&str] = &[MIGRATION_V1, MIGRATION_V2];

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
