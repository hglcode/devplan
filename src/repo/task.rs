use anyhow::Result;
use chrono::NaiveDate;
use rusqlite::types::ToSql;
use rusqlite::{Connection, OptionalExtension, params};

use crate::error::DpError;
use crate::models::{Priority, Resolution, Task, TaskStatus, TaskType, join_tags, split_tags};

const COLS: &str = "id, plan_id, type, title, description, status, resolution, priority, tags, time_spent, due_date, started_at, done_at, created_at, updated_at";

pub struct NewTask<'a> {
    pub title: &'a str,
    pub plan: Option<i64>,
    pub task_type: TaskType,
    pub due_date: Option<NaiveDate>,
    pub priority: Priority,
    pub tags: Vec<String>,
    pub description: &'a str,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TaskFilter {
    pub plan: Option<i64>,
    pub view: TaskView,
    pub overdue: bool,
    pub task_type: Option<TaskType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskView {
    /// status = 'todo' (historical `dp list` default)
    #[default]
    Todo,
    /// status = 'active'
    Active,
    /// everything not done yet (plan views)
    Pending,
    /// status = 'done'
    Done,
}

#[derive(Debug, Default, Clone)]
pub struct TaskUpdate {
    pub title: Option<String>,
    pub task_type: Option<TaskType>,
    pub due_date: Option<NaiveDate>,
    pub priority: Option<Priority>,
    pub status: Option<TaskStatus>,
    pub tags: Option<Vec<String>>,
    pub description: Option<String>,
    pub plan: Option<i64>,
}

pub fn add(conn: &Connection, t: &NewTask) -> Result<i64> {
    if let Some(plan_id) = t.plan {
        crate::repo::plan::get(conn, plan_id)?; // clean "plan #N not found" instead of raw FK error
    }
    conn.execute(
        "INSERT INTO tasks (plan_id, type, title, description, priority, due_date, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            t.plan,
            t.task_type.as_str(),
            t.title,
            t.description,
            t.priority.as_int(),
            t.due_date.map(|d| d.to_string()),
            join_tags(&t.tags),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> Result<Task> {
    conn.query_row(
        &format!("SELECT {COLS} FROM tasks WHERE id = ?1"),
        params![id],
        map_row,
    )
    .optional()? // real DB errors propagate; only a missing row becomes NotFound
    .ok_or(DpError::TaskNotFound(id))
    .map_err(Into::into)
}

pub fn list(conn: &Connection, f: TaskFilter) -> Result<Vec<Task>> {
    let mut sql = format!("SELECT {COLS} FROM tasks WHERE 1=1");
    let mut owned: Vec<Box<dyn ToSql>> = Vec::new();
    match f.view {
        TaskView::Todo => sql.push_str(" AND status = 'todo'"),
        TaskView::Active => sql.push_str(" AND status = 'active'"),
        TaskView::Pending => sql.push_str(" AND status != 'done'"),
        TaskView::Done => sql.push_str(" AND status = 'done'"),
    }
    if let Some(p) = f.plan {
        sql.push_str(" AND plan_id = ?");
        owned.push(Box::new(p));
    }
    if f.overdue {
        sql.push_str(" AND due_date IS NOT NULL AND due_date < date('now','localtime')");
    }
    if let Some(tt) = f.task_type {
        sql.push_str(" AND type = ?");
        owned.push(Box::new(tt.as_str()));
    }
    sql.push_str(" ORDER BY priority DESC, due_date IS NULL, due_date ASC, id ASC");

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
    if let Some(tsk) = &u.task_type
        && *tsk != existing.r#type
    {
        sets.push("type = ?".into());
        owned.push(Box::new(tsk.to_string()));
    }
    if let Some(due) = u.due_date
        && Some(due) != existing.due_date
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
    if let Some(desc) = &u.description
        && *desc != existing.description
    {
        sets.push("description = ?".into());
        owned.push(Box::new(desc.clone()));
    }
    if let Some(p) = u.plan {
        crate::repo::plan::get(conn, p)?; // clean error instead of FK violation
        if Some(p) != existing.plan_id {
            sets.push("plan_id = ?".into());
            owned.push(Box::new(p));
        }
    }
    if sets.is_empty() {
        return Ok(false);
    }

    owned.push(Box::new(id));
    let sql = format!("UPDATE tasks SET {} WHERE id = ?", sets.join(", "));
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
        "UPDATE tasks SET status = ?1, done_at = ?2 WHERE id = ?3",
        params![status.as_str(), done_at, id],
    )?;
    Ok(())
}

/// Marks a task finished with a non-default resolution.
pub fn set_resolution(conn: &Connection, id: i64, resolution: Resolution) -> Result<()> {
    let t = get(conn, id)?;
    if t.status == TaskStatus::Done && t.resolution == Some(resolution) {
        return Ok(()); // idempotent
    }
    conn.execute(
        "UPDATE tasks SET status = 'done', resolution = ?1, done_at = ?2 WHERE id = ?3",
        params![resolution.as_str(), now_ts(), id],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    let n = conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
    if n == 0 {
        return Err(DpError::TaskNotFound(id).into());
    }
    Ok(())
}

pub fn count_done(conn: &Connection, plan_id: i64) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM tasks WHERE plan_id = ?1 AND status = 'done'",
        params![plan_id],
        |r| r.get(0),
    )?)
}

fn now_ts() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<Task> {
    let status: String = row.get(5)?;
    let resolution: Option<String> = row.get(6)?;
    let prio: i64 = row.get(7)?;
    let tags: String = row.get(8)?;
    let typ: String = row.get(2)?;
    let due: Option<String> = row.get(10)?;
    let start: Option<String> = row.get(11)?;
    Ok(Task {
        id: row.get(0)?,
        plan_id: row.get(1)?,
        r#type: TaskType::from_label(&typ).unwrap_or_default(),
        title: row.get(3)?,
        description: row.get(4)?,
        status: TaskStatus::from_label(&status).unwrap_or_default(),
        resolution: resolution.and_then(|r| Resolution::from_label(&r)),
        priority: Priority::from_int(prio).unwrap_or_default(),
        tags: split_tags(&tags),
        time_spent: row.get(9)?,
        due_date: due.and_then(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok()),
        started_at: start,
        done_at: row.get(12)?,
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
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
            task_type: TaskType::Feature,
            due_date: None,
            priority: Priority::Normal,
            tags: vec![],
            description: "",
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
        set_status(&c, id, TaskStatus::Todo).unwrap();
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
                description: Some("new description".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(changed);
        let t = get(&c, id).unwrap();
        assert_eq!(t.priority, Priority::High);
        assert_eq!(t.tags.join(","), "a,b");
        assert_eq!(t.description, "new description");
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
                priority: Some(Priority::Normal),
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

    #[test]
    fn abandon_sets_resolution_and_stats_exclude_it() {
        let c = conn();
        let id = add(&c, &sample("t1")).unwrap();
        set_resolution(&c, id, Resolution::Abandoned).unwrap();
        let t = get(&c, id).unwrap();
        assert_eq!(t.resolution, Some(Resolution::Abandoned));
        // stats 的 done_total 不含它:
        let s = crate::repo::stats::get(&c).unwrap();
        assert_eq!(s.done_total, 0);
    }

    #[test]
    fn migration_v3_preserves_plan_linkage() {
        let c = Connection::open_in_memory().unwrap();
        // 停在 v2:
        c.execute_batch(include_str!("../../sql/v1.sql")).unwrap();
        c.execute_batch(include_str!("../../sql/v2.sql")).unwrap();
        // v2 形态塞数据(plan + 挂靠的 task):
        c.execute("INSERT INTO plans (title) VALUES ('victim')", [])
            .unwrap();
        c.execute(
            "INSERT INTO tasks (plan_id, title) VALUES ((SELECT MAX(id) FROM plans), 'linked')",
            [],
        )
        .unwrap();
        c.execute("PRAGMA user_version = 2", []).unwrap();
        // 全程迁移(v3):
        crate::db::migrate(&c).unwrap();
        // 断言:归属存活
        let pid: Option<i64> = c
            .query_row("SELECT plan_id FROM tasks WHERE title='linked'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(pid, Some(1));
    }

    #[test]
    fn updated_at_trigger_fires_on_raw_update() {
        let c = conn();
        let id = add(&c, &sample("t1")).unwrap();
        let before: String = c
            .query_row(
                "SELECT updated_at FROM tasks WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap();
        // 裸 SQL 改(绕过 repo 层——trigger 的主战场):
        std::thread::sleep(std::time::Duration::from_millis(1100)); // 跨秒
        c.execute(
            "UPDATE tasks SET title = 'renamed' WHERE id = ?1",
            params![id],
        )
        .unwrap();
        let after: String = c
            .query_row(
                "SELECT updated_at FROM tasks WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .unwrap();
        assert_ne!(before, after); // ★ trigger 刷新了时间戳
    }
}
