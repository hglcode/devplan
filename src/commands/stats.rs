use anyhow::Result;
use rusqlite::Connection;

use crate::cli::Format;

pub fn run(conn: &Connection, fmt: Format) -> Result<()> {
    let s = crate::repo::stats::get(conn)?;
    if fmt == Format::Json {
        println!("{}", serde_json::to_string_pretty(&s)?);
        return Ok(());
    }
    println!("=== dp stats ===");
    println!("{:<18}{}", "Active plans:", s.active_plans);
    println!("{:<18}{}", "Open:", s.open);
    println!("{:<18}{}", "Active:", s.active);
    println!("{:<18}{}", "Blocked:", s.blocked);
    println!("{:<18}{}", "Overdue:", s.overdue);
    println!("{:<18}{}", "Done today:", s.done_today);
    println!("{:<18}{}", "Done (7 days):", s.done_week);
    println!("{:<18}{}", "Done total:", s.done_total);
    Ok(())
}
