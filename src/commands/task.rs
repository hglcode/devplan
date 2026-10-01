use anyhow::Result;
use chrono::NaiveDate;
use rusqlite::Connection;

use crate::cli::Format;
use crate::commands::print_tasks;
use crate::models::{Priority, TaskStatus, TaskType, split_tags};
use crate::repo;
use crate::repo::task::{NewTask, TaskFilter, TaskUpdate, TaskView};

pub struct AddArgs {
    pub title: String,
    pub plan: Option<i64>,
    pub task_type: Option<TaskType>,
    pub due_date: Option<NaiveDate>,
    pub priority: Option<Priority>,
    pub tags: Option<String>,
    pub desc: Option<String>,
}

pub struct ModifyArgs {
    pub id: i64,
    pub title: Option<String>,
    pub task_type: Option<TaskType>,
    pub due_date: Option<NaiveDate>,
    pub prio: Option<Priority>,
    pub stat: Option<TaskStatus>,
    pub tags: Option<String>,
    pub desc: Option<String>,
}

pub fn add(conn: &Connection, args: AddArgs) -> Result<()> {
    let AddArgs {
        title,
        plan,
        task_type,
        due_date,
        priority,
        tags,
        desc,
    } = args;
    let id = repo::task::add(
        conn,
        &NewTask {
            title: &title,
            plan,
            task_type: task_type.unwrap_or_default(),
            due_date,
            priority: priority.unwrap_or_default(),
            tags: tags.map(|t| split_tags(&t)).unwrap_or_default(),
            description: desc.as_deref().unwrap_or(""),
        },
    )?;
    println!("Added todo #{id}: {title}");
    Ok(())
}

pub fn list(
    conn: &Connection,
    plan: Option<i64>,
    overdue: bool,
    active: bool,
    done: bool,
    fmt: Format,
) -> Result<()> {
    let view = if done {
        TaskView::Done
    } else if active {
        TaskView::Active
    } else {
        TaskView::Todo
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
        t.due_date.map_or_else(|| "-".into(), |d| d.to_string())
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
    if !t.description.is_empty() {
        println!("  Description:");
        for line in t.description.lines() {
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

pub fn modify(conn: &Connection, args: ModifyArgs) -> Result<()> {
    let ModifyArgs {
        id,
        title,
        task_type,
        due_date,
        prio,
        stat,
        tags,
        desc,
    } = args;
    let provided = title.is_some()
        || task_type.is_some()
        || due_date.is_some()
        || prio.is_some()
        || stat.is_some()
        || tags.is_some()
        || desc.is_some();
    let update = TaskUpdate {
        title,
        due_date,
        task_type,
        priority: prio,
        status: stat,
        tags: tags.map(|t| split_tags(&t)),
        description: desc,
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
