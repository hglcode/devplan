use anyhow::Result;
use chrono::NaiveDate;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::error::DpError;
use crate::models::{Plan, PlanStatus, Priority};

const COLS: &str =
    "id, title, description, status, priority, start_date, due_date, created_at, updated_at";

pub struct NewPlan {
    pub title: String,
    pub description: String,
    pub due_date: Option<NaiveDate>,
    pub priority: Priority,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanSummary {
    pub plan: Plan,
    pub open_tasks: i64,
}

pub fn add(conn: &Connection, p: &NewPlan) -> Result<i64> {
    conn.execute(
        "INSERT INTO plans (title, description, priority, due_date) VALUES (?1, ?2, ?3, ?4)",
        params![
            p.title,
            p.description,
            p.priority.as_int(),
            p.due_date.map(|d| d.to_string())
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> Result<Plan> {
    conn.query_row(
        &format!("SELECT {COLS} FROM plans WHERE id = ?1"),
        params![id],
        map_row,
    )
    .optional()?
    .ok_or(DpError::PlanNotFound(id))
    .map_err(Into::into)
}

pub fn list(conn: &Connection) -> Result<Vec<PlanSummary>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS},
                (SELECT COUNT(*) FROM tasks t WHERE t.plan_id = plans.id AND t.status != 'done')
         FROM plans
         ORDER BY (status = 'active') ASC, priority DESC, id DESC"
    ))?;
    let rows = stmt.query_map([], |row| {
        Ok(PlanSummary {
            plan: map_row(row)?,
            open_tasks: row.get(8)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn set_status(conn: &Connection, id: i64, status: PlanStatus) -> Result<()> {
    let n = conn.execute(
        "UPDATE plans SET status = ?1, updated_at = datetime('now','localtime') WHERE id = ?2",
        params![status.as_str(), id],
    )?;
    if n == 0 {
        return Err(DpError::PlanNotFound(id).into());
    }
    Ok(())
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<Plan> {
    let status: String = row.get(3)?;
    let prio: i64 = row.get(4)?;
    let due: Option<String> = row.get(5)?;
    let start: Option<String> = row.get(6)?;
    Ok(Plan {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        status: PlanStatus::from_label(&status).unwrap_or(PlanStatus::Active),
        priority: Priority::from_int(prio).unwrap_or_default(),
        due_date: due.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        started_at: start.and_then(|s: String| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}
