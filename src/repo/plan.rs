use anyhow::Result;
use chrono::NaiveDate;
use rusqlite::types::ToSql;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::error::DpError;
use crate::models::{Plan, PlanStatus, Priority};

const COLS: &str = "id, adr_id, title, description, status, priority, start_date, due_date, created_at, updated_at";
const COLS_P: &str = "p.id, p.adr_id, p.title, p.description, p.status, p.priority, p.start_date, p.due_date, p.created_at, p.updated_at";

pub struct NewPlan {
    pub title: String,
    pub adr_id: Option<i64>,
    pub description: String,
    pub start_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub priority: Priority,
}

#[derive(Debug, Default, Clone)]
pub struct PlanUpdate {
    pub title: Option<String>,
    pub description: Option<String>,
    pub priority: Option<Priority>,
    pub start_date: Option<NaiveDate>,
    pub due_date: Option<NaiveDate>,
    pub adr_id: Option<i64>,
    pub status: Option<PlanStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanSummary {
    pub plan: Plan,
    pub total_tasks: i64,
    pub done_tasks: i64,
    pub progress_pct: f64,
}

pub fn add(conn: &Connection, p: &NewPlan) -> Result<i64> {
    conn.execute(
        "INSERT INTO plans (adr_id, title, description, priority, start_date, due_date)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            p.adr_id,
            p.title,
            p.description,
            p.priority.as_int(),
            p.start_date.map(|d| d.to_string()),
            p.due_date.map(|d| d.to_string()),
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
        "SELECT {COLS_P}, v.total_tasks, v.done_tasks, v.progress_pct
         FROM plans p JOIN v_plan_progress v ON v.id = p.id
         ORDER BY (p.status = 'open') DESC, p.priority DESC, p.id DESC",
    ))?;
    let rows = stmt.query_map([], |row| {
        Ok(PlanSummary {
            plan: map_row(row)?,
            total_tasks: row.get(10)?,
            done_tasks: row.get(11)?,
            progress_pct: row.get(12)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Applies the non-None fields. Returns false when there was nothing to change.
pub fn update(conn: &Connection, id: i64, u: &PlanUpdate) -> Result<bool> {
    let existing = get(conn, id)?;

    let mut sets: Vec<String> = Vec::new();
    let mut owned: Vec<Box<dyn ToSql>> = Vec::new();

    if let Some(title) = &u.title
        && *title != existing.title
    {
        sets.push("title = ?".into());
        owned.push(Box::new(title.clone()));
    }
    if let Some(desc) = &u.description
        && *desc != existing.description
    {
        sets.push("description = ?".into());
        owned.push(Box::new(desc.clone()));
    }
    if let Some(p) = u.priority
        && p != existing.priority
    {
        sets.push("priority = ?".into());
        owned.push(Box::new(p.as_int()));
    }
    if let Some(s) = u.start_date
        && Some(s) != existing.start_date
    {
        sets.push("start_date = ?".into());
        owned.push(Box::new(s.to_string()));
    }
    if let Some(d) = u.due_date
        && Some(d) != existing.due_date
    {
        sets.push("due_date = ?".into());
        owned.push(Box::new(d.to_string()));
    }
    if let Some(a) = u.adr_id {
        crate::repo::adr::get(conn, a)?; // clean "ADR #N not found" instead of raw FK error
        if Some(a) != existing.adr_id {
            sets.push("adr_id = ?".into());
            owned.push(Box::new(a));
        }
    }
    if let Some(s) = u.status
        && s != existing.status
    {
        sets.push("status = ?".into());
        owned.push(Box::new(s.as_str().to_string()));
    }

    if sets.is_empty() {
        return Ok(false);
    }

    owned.push(Box::new(id));
    let sql = format!("UPDATE plans SET {} WHERE id = ?", sets.join(", "));
    let args: Vec<&dyn ToSql> = owned.iter().map(std::convert::AsRef::as_ref).collect();
    conn.execute(&sql, args.as_slice())?;
    Ok(true)
}

pub fn set_status(conn: &Connection, id: i64, status: PlanStatus) -> Result<()> {
    let n = conn.execute(
        "UPDATE plans SET status = ?1 WHERE id = ?2",
        params![status.as_str(), id],
    )?;
    if n == 0 {
        return Err(DpError::PlanNotFound(id).into());
    }
    Ok(())
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<Plan> {
    let status: String = row.get(4)?;
    let prio: i64 = row.get(5)?;
    let start: Option<String> = row.get(6)?;
    let due: Option<String> = row.get(7)?;
    Ok(Plan {
        id: row.get(0)?,
        adr_id: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        status: PlanStatus::from_label(&status).unwrap_or(PlanStatus::Open),
        priority: Priority::from_int(prio).unwrap_or_default(),
        start_date: start.and_then(|s: String| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        due_date: due.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrate;
    use crate::repo::adr::{self, NewAdr};
    use rusqlite::Connection;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrate(&c).unwrap();
        c
    }

    #[test]
    fn plan_list_works_on_fresh_db() {
        let c = conn();
        let _id = add(
            &c,
            &NewPlan {
                title: "p".into(),
                adr_id: None,
                description: "".into(),
                start_date: None,
                due_date: None,
                priority: Priority::Normal,
            },
        )
        .unwrap();
        let s = list(&c).unwrap(); // ← 不是 plan::list
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].total_tasks, 0); // 顺手断言视图列读取(空计划的 progress 边界)
    }

    #[test]
    fn plan_update_partial_fields_and_noop() {
        let c = conn();
        let id = add(
            &c,
            &NewPlan {
                title: "p".into(),
                adr_id: None,
                description: "d".into(),
                start_date: None,
                due_date: None,
                priority: Priority::Normal,
            },
        )
        .unwrap();
        let u = PlanUpdate {
            title: Some("renamed".into()),
            priority: Some(Priority::Urgent),
            ..Default::default()
        };
        assert!(update(&c, id, &u).unwrap());
        let p = get(&c, id).unwrap();
        assert_eq!(p.title, "renamed");
        assert_eq!(p.priority, Priority::Urgent);
        assert_eq!(p.description, "d"); // 未给,原样
        // 同值再跑 → false,无写入
        assert!(!update(&c, id, &u).unwrap());
        // 零字段 → false
        assert!(!update(&c, id, &PlanUpdate::default()).unwrap());
    }

    #[test]
    fn plan_update_status_window_and_adr_link() {
        let c = conn();
        let id = add(
            &c,
            &NewPlan {
                title: "p".into(),
                adr_id: None,
                description: String::new(),
                start_date: None,
                due_date: None,
                priority: Priority::Normal,
            },
        )
        .unwrap();
        let a = adr::add(
            &c,
            &NewAdr {
                title: "A",
                context: "ctx",
                decision: "dec",
                consequence: "",
            },
        )
        .unwrap();
        let u = PlanUpdate {
            adr_id: Some(a),
            status: Some(PlanStatus::Archived),
            start_date: Some(NaiveDate::from_ymd_opt(2026, 11, 1).unwrap()),
            due_date: Some(NaiveDate::from_ymd_opt(2026, 11, 30).unwrap()),
            ..Default::default()
        };
        assert!(update(&c, id, &u).unwrap());
        let p = get(&c, id).unwrap();
        assert_eq!(p.status, PlanStatus::Archived);
        assert_eq!(p.adr_id, Some(a));
        assert_eq!(
            p.start_date.map(|d| d.to_string()),
            Some("2026-11-01".into())
        );

        // 悬空 ADR 引用 → 干净报错
        let bad = PlanUpdate {
            adr_id: Some(9999),
            ..Default::default()
        };
        assert!(update(&c, id, &bad).is_err());
    }
}
