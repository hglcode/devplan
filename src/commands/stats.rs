use anyhow::Result;
use rusqlite::Connection;

pub fn run(conn: &Connection) -> Result<()> {
    let s = crate::repo::stats::get(conn)?;
    println!("=== dp stats ===");
    println!("{:<18}{}", "Active plans:", s.active_plans);
    println!("{:<18}{}", "Open:", s.open);
    println!("{:<18}{}", "In progress:", s.in_progress);
    println!("{:<18}{}", "Blocked:", s.blocked);
    println!("{:<18}{}", "Overdue:", s.overdue);
    println!("{:<18}{}", "Done today:", s.done_today);
    println!("{:<18}{}", "Done (7 days):", s.done_week);
    println!("{:<18}{}", "Done total:", s.done_total);
    Ok(())
}
