use anyhow::Result;
use chrono::NaiveDate;
use rusqlite::Connection;

use crate::cli::Format;
use crate::commands::print_tasks;
use crate::models::{Priority, TaskStatus, split_tags};
use crate::repo;
use crate::repo::task::{NewTask, TaskFilter, TaskUpdate, TaskView};

pub fn add(
    conn: &Connection,
    title: &str,
    plan: Option<i64>,
    due: Option<NaiveDate>,
    priority: Option<Priority>,
    tags: Option<String>,
    detail: Option<String>,
) -> Result<()> {
    let id = repo::task::add(
        conn,
        &NewTask {
            title,
            plan,
            due,
            priority: priority.unwrap_or_default(),
            tags: tags.map(|t| split_tags(&t)).unwrap_or_default(),
            detail: detail.as_deref().unwrap_or(""),
        },
    )?;
    println!("Added todo #{id}: {title}");
    Ok(())
}

pub fn list(
    conn: &Connection,
    plan: Option<i64>,
    overdue: bool,
    all: bool,
    done: bool,
    fmt: Format,
) -> Result<()> {
    let view = if done {
        TaskView::Done
    } else if all {
        TaskView::Active
    } else {
        TaskView::Open
    };
    let tasks = repo::task::list(
        conn,
        TaskFilter {
            plan,
            view,
            overdue,
        },
    )?;
    print_tasks(&tasks, fmt)
}

pub fn show(conn: &Connection, id: i64, fmt: Format) -> Result<()> {
    let t = repo::task::get(conn, id)?;
    if fmt == Format::Json {
        println!("{}", serde_json::to_string_pretty(&t)?);
        return Ok(());
    }
    println!("Todo #{}", t.id);
    println!("  Title:    {}", t.title);
    println!(
        "  Plan:     {}",
        t.plan_id.map_or_else(|| "-".into(), |p| p.to_string())
    );
    println!("  Status:   {}", t.status);
    println!("  Priority: {}", t.priority);
    println!(
        "  Due:      {}",
        t.due.map_or_else(|| "-".into(), |d| d.to_string())
    );
    println!(
        "  Tags:     {}",
        if t.tags.is_empty() {
            "-".to_string()
        } else {
            t.tags.join(", ")
        }
    );
    println!("  Created:  {}", t.created_at);
    println!("  Updated:  {}", t.updated_at);
    if let Some(done_at) = &t.done_at {
        println!("  Done at:  {done_at}");
    }
    if !t.detail.is_empty() {
        println!("  Detail:");
        for line in t.detail.lines() {
            println!("    {line}");
        }
    }
    Ok(())
}

pub fn done(conn: &Connection, id: i64) -> Result<()> {
    let t = repo::task::get(conn, id)?;
    if t.status == TaskStatus::Done {
        println!("Todo #{id} is already done");
        return Ok(());
    }
    repo::task::set_status(conn, id, TaskStatus::Done)?;
    println!("Done: todo #{id} — {}", t.title);
    Ok(())
}

pub fn rm(conn: &Connection, id: i64) -> Result<()> {
    repo::task::delete(conn, id)?;
    println!("Deleted todo #{id}");
    Ok(())
}

pub fn modify(
    conn: &Connection,
    id: i64,
    title: Option<String>,
    due: Option<NaiveDate>,
    priority: Option<Priority>,
    status: Option<TaskStatus>,
    tags: Option<String>,
    detail: Option<String>,
) -> Result<()> {
    let provided = title.is_some()
        || due.is_some()
        || priority.is_some()
        || status.is_some()
        || tags.is_some()
        || detail.is_some();
    let update = TaskUpdate {
        title,
        due,
        priority,
        status,
        tags: tags.map(|t| split_tags(&t)),
        detail,
    };
    if provided && repo::task::update(conn, id, &update)? {
        println!("Updated todo #{id}");
    } else if provided {
        println!("Nothing to modify: values already as requested"); // 有 flag 但值未变
    } else {
        println!("Nothing to modify: no fields given"); // 零 flag
    }
    Ok(())
}
