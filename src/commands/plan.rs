use anyhow::Result;
use chrono::NaiveDate;
use comfy_table::{Table, presets::UTF8_FULL};
use rusqlite::Connection;

use crate::cli::PlanCommands;
use crate::models::{PlanStatus, Priority};
use crate::repo;
use crate::repo::plan::NewPlan;
use crate::repo::task::{TaskFilter, TaskView};

pub fn run(conn: &Connection, cmd: PlanCommands) -> Result<()> {
    match cmd {
        PlanCommands::Add {
            title,
            due,
            priority,
            description,
        } => add(conn, &title, due, priority, description),
        PlanCommands::List => list(conn),
        PlanCommands::Show { id } => show(conn, id),
        PlanCommands::Done { id } => done(conn, id),
    }
}

fn add(
    conn: &Connection,
    title: &str,
    due: Option<NaiveDate>,
    priority: Option<Priority>,
    description: Option<String>,
) -> Result<()> {
    let id = repo::plan::add(
        conn,
        &NewPlan {
            title: title.to_string(),
            description: description.unwrap_or_default(),
            due,
            priority: priority.unwrap_or_default(),
        },
    )?;
    println!("Added plan #{id}: {title}");
    Ok(())
}

fn list(conn: &Connection) -> Result<()> {
    let summaries = repo::plan::list(conn)?;
    if summaries.is_empty() {
        println!("(no plans)");
        return Ok(());
    }
    let mut table = Table::new();
    table.load_style(UTF8_FULL);
    table.set_header(vec!["ID", "Status", "Prio", "Due", "Open#", "Title"]);
    for s in &summaries {
        let p = &s.plan;
        table.add_row(vec![
            p.id.to_string(),
            p.status.to_string(),
            p.priority.to_string(),
            p.due.map(|d| d.to_string()).unwrap_or_default(),
            s.open_tasks.to_string(),
            p.title.clone(),
        ]);
    }
    println!("{table}");
    Ok(())
}

fn show(conn: &Connection, id: i64) -> Result<()> {
    let p = repo::plan::get(conn, id)?;
    println!("Plan #{}", p.id);
    println!("  Title:       {}", p.title);
    println!("  Status:      {}", p.status);
    println!("  Priority:    {}", p.priority);
    println!(
        "  Due:         {}",
        p.due.map_or_else(|| "-".into(), |d| d.to_string())
    );
    println!("  Created:     {}", p.created_at);
    if !p.description.is_empty() {
        println!("  Description: {}", p.description);
    }

    let tasks = repo::task::list(
        conn,
        TaskFilter {
            plan: Some(id),
            view: TaskView::Active,
            overdue: false,
        },
    )?;
    println!("\nOpen todos:");
    if tasks.is_empty() {
        println!("  (none)");
    } else {
        let mut table = Table::new();
        table.load_style(UTF8_FULL);
        table.set_header(vec!["ID", "Status", "Prio", "Due", "Title"]);
        for t in &tasks {
            table.add_row(vec![
                t.id.to_string(),
                t.status.to_string(),
                t.priority.to_string(),
                t.due.map(|d| d.to_string()).unwrap_or_default(),
                t.title.clone(),
            ]);
        }
        println!("{table}");
    }

    let done = repo::task::count_done(conn, id)?;
    println!("\nDone in this plan: {done}");
    Ok(())
}

fn done(conn: &Connection, id: i64) -> Result<()> {
    repo::plan::set_status(conn, id, PlanStatus::Done)?;
    println!("Marked plan #{id} done");
    Ok(())
}
