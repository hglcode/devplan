use anyhow::Result;
use chrono::NaiveDate;
use rusqlite::Connection;
use std::io::Write;

use crate::cli::{Format, TaskCommands};
use crate::commands::print_tasks;
use crate::models::{Priority, Resolution, TaskStatus, TaskType, split_tags};
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
    pub plan: Option<i64>,
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
    println!("Added task #{id}: {title}");
    Ok(())
}

pub fn list(
    conn: &Connection,
    plan: Option<i64>,
    overdue: bool,
    active: bool,
    done: bool,
    task_type: Option<TaskType>,
    fmt: Format,
) -> Result<()> {
    let view = if done {
        TaskView::Done
    } else if active {
        TaskView::Active
    } else {
        TaskView::Pending
    };
    let tasks = repo::task::list(
        conn,
        TaskFilter {
            plan,
            view,
            overdue,
            task_type,
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
    println!("Task #{}", t.id);
    println!("  Type:     {}", t.r#type);
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
    if let Some(r) = &t.resolution {
        let mut line = format!("  Resolution: {}", r);
        if let Some(of) = t.duplicate_of {
            line.push_str(&format!(" (of #{of})")); // 指向
        }
        println!("{line}");
        if let Some(note) = &t.resolution_note
            && !note.is_empty()
        {
            println!("    Note:     {note}");
        }
    }
    if !t.description.is_empty() {
        println!("  Description:");
        for line in t.description.lines() {
            println!("    {line}");
        }
    }
    Ok(())
}

pub fn done(conn: &Connection, id: i64, note: Option<String>) -> Result<()> {
    let t = repo::task::get(conn, id)?;
    if t.status == TaskStatus::Done {
        println!("Task #{id} is already done");
        return Ok(());
    }
    repo::task::set_resolution(conn, id, Resolution::Done, note.as_deref(), None)?;
    println!("Done: task #{id} — {}", t.title);
    Ok(())
}

pub fn abandon(conn: &Connection, id: i64, note: Option<String>) -> Result<()> {
    let t = repo::task::get(conn, id)?;
    repo::task::set_resolution(conn, id, Resolution::Abandoned, note.as_deref(), None)?;
    println!("Abandoned: task #{id} — {}", t.title);
    Ok(())
}

pub fn duplicate(conn: &Connection, id: i64, note: Option<String>, of: Option<i64>) -> Result<()> {
    let t = repo::task::get(conn, id)?;
    repo::task::set_resolution(conn, id, Resolution::Duplicated, note.as_deref(), of)?;
    println!("Closed as duplicate: task #{id} — {}", t.title);
    Ok(())
}

pub fn rm(conn: &Connection, id: i64, force: bool) -> Result<()> {
    let t = repo::task::get(conn, id)?;

    if !force {
        println!("Task #{} \"{}\"", t.id, t.title);
        println!(
            "  status: {}, created {}, plan: {}",
            t.status,
            t.created_at,
            t.plan_id.map_or("none".to_string(), |p| p.to_string())
        );
        print!("Delete permanently? [y/N] ");
        std::io::stdout().flush()?;
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        if !answer.trim().eq_ignore_ascii_case("y") {
            println!("Aborted");
            return Ok(());
        }
    }
    repo::task::delete(conn, id)?;
    println!("Deleted task #{id}");
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
        plan,
    } = args;
    let provided = title.is_some()
        || task_type.is_some()
        || due_date.is_some()
        || prio.is_some()
        || stat.is_some()
        || tags.is_some()
        || desc.is_some()
        || plan.is_some();
    let update = TaskUpdate {
        title,
        due_date,
        task_type,
        priority: prio,
        status: stat,
        tags: tags.map(|t| split_tags(&t)),
        description: desc,
        plan,
    };
    if provided && repo::task::update(conn, id, &update)? {
        println!("Updated task #{id}");
    } else if provided {
        println!("Nothing to modify: values already as requested"); // 有 flag 但值未变
    } else {
        println!("Nothing to modify: no fields given"); // 零 flag
    }
    Ok(())
}

pub fn run(conn: &Connection, cmd: TaskCommands, fmt: Format) -> Result<()> {
    match cmd {
        TaskCommands::Add {
            title,
            plan,
            due_date,
            priority,
            tags,
            description,
            task_type,
        } => add(
            conn,
            AddArgs {
                title,
                plan,
                task_type,
                due_date,
                priority,
                tags,
                desc: description,
            },
        ),
        TaskCommands::List {
            plan,
            overdue,
            active,
            done,
            task_type,
        } => list(conn, plan, overdue, active, done, task_type, fmt),
        TaskCommands::Show { id } => show(conn, id, fmt),
        TaskCommands::Done { id, note } => done(conn, id, note),
        TaskCommands::Abandon { id, note } => abandon(conn, id, note),
        TaskCommands::Duplicate { id, note, of } => duplicate(conn, id, note, of),
        TaskCommands::Rm { id, force } => rm(conn, id, force),
        TaskCommands::Mod {
            id,
            title,
            due_date,
            priority,
            status,
            tags,
            description,
            task_type,
            plan,
        } => modify(
            conn,
            ModifyArgs {
                id,
                title,
                task_type,
                due_date,
                prio: priority,
                stat: status,
                tags,
                desc: description,
                plan,
            },
        ),
    }
}
