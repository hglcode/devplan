use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct Stats {
    pub active_plans: i64,
    pub open: i64,
    pub active: i64,
    pub blocked: i64,
    pub overdue: i64,
    pub done_today: i64,
    pub done_week: i64,
    pub done_total: i64,
}

pub fn get(conn: &Connection) -> Result<Stats> {
    let s = conn.query_row(
        "SELECT
            (SELECT COUNT(*) FROM plans WHERE status = 'open'),
            COALESCE(SUM(status = 'todo'), 0),
            COALESCE(SUM(status = 'active'), 0),
            COALESCE(SUM(status = 'blocked'), 0),
            COALESCE(SUM(status != 'done' AND due_date IS NOT NULL
                        AND due_date < date('now','localtime')), 0),
            COALESCE(SUM(status = 'done' AND (resolution IS NULL OR resolution != 'abandoned')
                        AND date(done_at) = date('now','localtime')), 0),
            COALESCE(SUM(status = 'done' AND (resolution IS NULL OR resolution != 'abandoned')
                        AND done_at >= datetime('now','localtime','-7 days')), 0),
            COALESCE(SUM(status = 'done' AND (resolution IS NULL OR resolution != 'abandoned')), 0)
            FROM tasks",
        [],
        |r| {
            Ok(Stats {
                active_plans: r.get(0)?,
                open: r.get(1)?,
                active: r.get(2)?,
                blocked: r.get(3)?,
                overdue: r.get(4)?,
                done_today: r.get(5)?,
                done_week: r.get(6)?,
                done_total: r.get(7)?,
            })
        },
    )?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*; // get/Stats 进来
    use crate::db::migrate;
    use crate::models::{Priority, TaskStatus, TaskType};
    use crate::repo::task::{self, NewTask};
    use rusqlite::Connection;

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
    fn stats_counts_all_status_axes() {
        let c = conn();
        task::add(&c, &sample("t-todo")).unwrap(); // #1 todo
        let id2 = task::add(&c, &sample("t-active")).unwrap();
        task::start(&c, id2).unwrap(); // active
        let id3 = task::add(&c, &sample("t-blocked")).unwrap();
        task::set_status(&c, id3, TaskStatus::Blocked).unwrap();

        let s = get(&c).unwrap(); // super::get,裸调
        assert_eq!(s.open, 1);
        assert_eq!(s.active, 1); // ★ bug 守护
        assert_eq!(s.blocked, 1);
        assert_eq!(s.done_total, 0);
    }
}
