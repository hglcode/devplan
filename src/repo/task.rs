use anyhow::Result;
use chrono::NaiveDate;
use rusqlite::types::ToSql;
use rusqlite::{Connection, OptionalExtension, params};

use crate::error::DpError;
use crate::models::{Priority, Task, TaskStatus, join_tags, split_tags};

const COLS: &str =
    "id, plan_id, title, detail, status, priority, due_date, tags, created_at, updated_at, done_at";

pub struct NewTask<'a> {
    pub title: &'a str,
    pub plan: Option<i64>,
    pub due: Option<NaiveDate>,
    pub priority: Priority,
    pub tags: Vec<String>,
    pub detail: &'a str,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TaskFilter {
    pub plan: Option<i64>,
    pub view: TaskView,
    pub overdue: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskView {
    /// status = 'open' (historical `dp list` default)
    #[default]
    Open,
    /// everything not done yet
    Active,
    /// status = 'done'
    Done,
}

#[derive(Debug, Default, Clone)]
pub struct TaskUpdate {
    pub title: Option<String>,
    pub due: Option<NaiveDate>,
    pub priority: Option<Priority>,
    pub status: Option<TaskStatus>,
    pub tags: Option<Vec<String>>,
    pub detail: Option<String>,
}

pub fn add(conn: &Connection, t: &NewTask) -> Result<i64> {
    if let Some(plan_id) = t.plan {
        crate::repo::plan::get(conn, plan_id)?; // clean "plan #N not found" instead of raw FK error
    }
    conn.execute(
        "INSERT INTO todos (plan_id, title, detail, priority, due_date, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            t.plan,
            t.title,
            t.detail,
            t.priority.as_int(),
            t.due.map(|d| d.to_string()),
            join_tags(&t.tags)
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> Result<Task> {
    conn.query_row(
        &format!("SELECT {COLS} FROM todos WHERE id = ?1"),
        params![id],
        map_row,
    )
    .optional()? // real DB errors propagate; only a missing row becomes NotFound
    .ok_or(DpError::TaskNotFound(id))
    .map_err(Into::into)
}

pub fn list(conn: &Connection, f: TaskFilter) -> Result<Vec<Task>> {
    let mut sql = format!("SELECT {COLS} FROM todos WHERE 1=1");
    let mut owned: Vec<Box<dyn ToSql>> = Vec::new();
    match f.view {
        TaskView::Open => sql.push_str(" AND status = 'open'"),
        TaskView::Active => sql.push_str(" AND status != 'done'"),
        TaskView::Done => sql.push_str(" AND status = 'done'"),
    }
    if let Some(p) = f.plan {
        sql.push_str(" AND plan_id = ?");
        owned.push(Box::new(p));
    }
    if f.overdue {
        sql.push_str(" AND due_date IS NOT NULL AND due_date < date('now','localtime')");
    }
    sql.push_str(" ORDER BY priority ASC, due_date IS NULL, due_date ASC, id ASC");

    let args: Vec<&dyn ToSql> = owned.iter().map(std::convert::AsRef::as_ref).collect();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(args.as_slice(), map_row)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Applies the non-None fields. Returns false when there was nothing to change.
pub fn update(conn: &Connection, id: i64, u: &TaskUpdate) -> Result<bool> {
    let existing = get(conn, id)?;

    let mut sets: Vec<String> = Vec::new();
    let mut owned: Vec<Box<dyn ToSql>> = Vec::new();

    if let Some(title) = &u.title
        && *title != existing.title
    {
        sets.push("title = ?".into());
        owned.push(Box::new(title.clone()));
    }
    if let Some(due) = u.due
        && Some(due) != existing.due
    {
        sets.push("due_date = ?".into());
        owned.push(Box::new(due.to_string()));
    }
    if let Some(p) = u.priority
        && p != existing.priority
    {
        sets.push("priority = ?".into());
        owned.push(Box::new(p.as_int()));
    }
    if let Some(s) = u.status
        && s != existing.status
    {
        sets.push("status = ?".into());
        owned.push(Box::new(s.as_str().to_string()));
        match s {
            TaskStatus::Done => {
                sets.push("done_at = ?".into());
                owned.push(Box::new(now_ts()));
            }
            _ => sets.push("done_at = NULL".into()),
        }
    }
    if let Some(tags) = &u.tags
        && *tags != existing.tags
    {
        sets.push("tags = ?".into());
        owned.push(Box::new(join_tags(tags)));
    }
    if let Some(detail) = &u.detail
        && *detail != existing.detail
    {
        sets.push("detail = ?".into());
        owned.push(Box::new(detail.clone()));
    }
    if sets.is_empty() {
        return Ok(false);
    }

    sets.push("updated_at = datetime('now','localtime')".into());
    owned.push(Box::new(id));
    let sql = format!("UPDATE todos SET {} WHERE id = ?", sets.join(", "));
    let args: Vec<&dyn ToSql> = owned.iter().map(std::convert::AsRef::as_ref).collect();
    conn.execute(&sql, args.as_slice())?;
    Ok(true)
}

pub fn set_status(conn: &Connection, id: i64, status: TaskStatus) -> Result<()> {
    let t = get(conn, id)?;
    if t.status == status {
        return Ok(()); // idempotent
    }
    let done_at = match status {
        TaskStatus::Done => Some(now_ts()),
        _ => None,
    };
    conn.execute(
        "UPDATE todos SET status = ?1, done_at = ?2, updated_at = datetime('now','localtime') WHERE id = ?3",
        params![status.as_str(), done_at, id],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    let n = conn.execute("DELETE FROM todos WHERE id = ?1", params![id])?;
    if n == 0 {
        return Err(DpError::TaskNotFound(id).into());
    }
    Ok(())
}

pub fn count_done(conn: &Connection, plan_id: i64) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM todos WHERE plan_id = ?1 AND status = 'done'",
        params![plan_id],
        |r| r.get(0),
    )?)
}

fn now_ts() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<Task> {
    let status: String = row.get(4)?;
    let prio: i64 = row.get(5)?;
    let due: Option<String> = row.get(6)?;
    let tags: String = row.get(7)?;
    Ok(Task {
        id: row.get(0)?,
        plan_id: row.get(1)?,
        title: row.get(2)?,
        detail: row.get(3)?,
        // unknown labels only occur in hand-edited databases
        status: TaskStatus::from_label(&status).unwrap_or(TaskStatus::Open),
        priority: Priority::from_int(prio).unwrap_or_default(),
        due: due.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        tags: split_tags(&tags),
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        done_at: row.get(10)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrate;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrate(&c).unwrap();
        c
    }

    fn sample(title: &str) -> NewTask<'_> {
        NewTask {
            title,
            plan: None,
            due: None,
            priority: Priority::Medium,
            tags: vec![],
            detail: "",
        }
    }

    #[test]
    fn done_sets_done_at_and_reopen_clears_it() {
        let c = conn();
        let id = add(&c, &sample("t1")).unwrap();
        set_status(&c, id, TaskStatus::Done).unwrap();
        let t = get(&c, id).unwrap();
        assert_eq!(t.status, TaskStatus::Done);
        assert!(t.done_at.is_some());
        set_status(&c, id, TaskStatus::Open).unwrap();
        assert!(get(&c, id).unwrap().done_at.is_none());
    }

    #[test]
    fn update_partial_fields() {
        let c = conn();
        let id = add(&c, &sample("t1")).unwrap();
        let changed = update(
            &c,
            id,
            &TaskUpdate {
                priority: Some(Priority::High),
                tags: Some(vec!["a".into(), "b".into()]),
                detail: Some("new detail".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(changed);
        let t = get(&c, id).unwrap();
        assert_eq!(t.priority, Priority::High);
        assert_eq!(t.tags.join(","), "a,b");
        assert_eq!(t.detail, "new detail");
        assert!(!update(&c, id, &TaskUpdate::default()).unwrap());
    }

    #[test]
    fn missing_task_is_a_clean_error() {
        let c = conn();
        let err = get(&c, 123).unwrap_err();
        assert!(err.to_string().contains("#123 not found"));
    }

    #[test]
    fn update_same_value_reports_no_change() {
        let c = conn();
        let id = add(&c, &sample("t1")).unwrap();
        // priority 已是 Medium,再设 Medium:
        let changed = update(
            &c,
            id,
            &TaskUpdate {
                priority: Some(Priority::Medium),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(!changed);
    }

    #[test]
    fn update_no_fields_reports_no_change() {
        let c = conn();
        let id = add(&c, &sample("t1")).unwrap();
        assert!(!update(&c, id, &TaskUpdate::default()).unwrap());
    }
}
