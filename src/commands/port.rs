//! dp export / dp import — 命令层:文件 IO、版本门、确认提示。
use std::io::Write;

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::db::SCHEMA_VERSION;
use crate::models::Snapshot;
use crate::repo;

pub fn export(conn: &Connection) -> Result<()> {
    let snap = repo::port::dump(conn)?;
    println!("{}", serde_json::to_string_pretty(&snap)?);
    Ok(())
}

pub fn import(conn: &Connection, file: &str, force: bool) -> Result<()> {
    let text = std::fs::read_to_string(file).with_context(|| format!("cannot read '{file}'"))?;
    let snap: Snapshot = serde_json::from_str(&text)
        .with_context(|| format!("'{file}' is not a dp export (invalid JSON)"))?;

    // 版本门:import 只吃与本二进制同 schema 的文件(向前向后都拒)
    if snap.schema_version != SCHEMA_VERSION {
        anyhow::bail!(
            "snapshot schema_version {} does not match this dp (schema {SCHEMA_VERSION}); \
             use the dp version that produced the export",
            snap.schema_version
        );
    }

    let (cur_plans, cur_tasks, cur_adrs) = repo::port::counts(conn)?;
    if !force {
        println!("Current database: {cur_plans} plan(s), {cur_tasks} task(s), {cur_adrs} ADR(s)");
        println!(
            "Snapshot '{file}': {} plan(s), {} task(s), {} ADR(s)",
            snap.plans.len(),
            snap.tasks.len(),
            snap.adrs.len()
        );
        print!("This REPLACES the current database. Continue? [y/N] ");
        std::io::stdout().flush()?;
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if !answer.trim().eq_ignore_ascii_case("y") {
            println!("Aborted");
            return Ok(());
        }
    }

    repo::port::restore(conn, &snap)?;
    println!(
        "Imported {} plan(s), {} task(s), {} ADR(s) from '{file}' (database replaced)",
        snap.plans.len(),
        snap.tasks.len(),
        snap.adrs.len()
    );
    Ok(())
}
