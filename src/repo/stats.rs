use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct Stats {
    pub active_plans: i64,
    pub open: i64,
    pub in_progress: i64,
    pub blocked: i64,
    pub overdue: i64,
    pub done_today: i64,
    pub done_week: i64,
    pub done_total: i64,
}

pub fn get(conn: &Connection) -> Result<Stats> {
    let s = conn.query_row(
        "SELECT
            (SELECT COUNT(*) FROM plans WHERE status = 'active'),
            COALESCE(SUM(status = 'open'), 0),
            COALESCE(SUM(status = 'in_progress'), 0),
            COALESCE(SUM(status = 'blocked'), 0),
            COALESCE(SUM(status != 'done' AND due_date IS NOT NULL
                          AND due_date < date('now','localtime')), 0),
            COALESCE(SUM(status = 'done' AND date(done_at) = date('now','localtime')), 0),
            COALESCE(SUM(status = 'done'
                          AND done_at >= datetime('now','localtime','-7 days')), 0),
            COALESCE(SUM(status = 'done'), 0)
         FROM todos",
        [],
        |r| {
            Ok(Stats {
                active_plans: r.get(0)?,
                open: r.get(1)?,
                in_progress: r.get(2)?,
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
