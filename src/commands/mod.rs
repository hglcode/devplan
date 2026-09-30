pub mod decision;
pub mod plan;
pub mod stats;
pub mod task;

use anyhow::Result;
use comfy_table::presets::UTF8_FULL;
use rusqlite::Connection;

use crate::cli::{Commands, Format};
use crate::models::Task;

fn print_tasks(tasks: &[Task], fmt: Format) -> Result<()> {
    match fmt {
        Format::Json => println!("{}", serde_json::to_string_pretty(tasks)?),
        Format::Markdown => {
            println!("| ID | Plan | Prio | Due | Status | Title | Tags |");
            println!("| -- | ---- | ---- | --- | -----  | ----- | ---- |");
            for t in tasks {
                println!(
                    "| {} | {} | {} | {} | {} | {} | {} |",
                    t.id,
                    t.plan_id.map_or_else(|| "-".into(), |p| p.to_string()),
                    t.priority,
                    t.due.map_or_else(|| "-".into(), |d| d.to_string()),
                    t.status,
                    t.title.replace('|', "\\|"),
                    t.tags.join(", ")
                );
            }
        }
        Format::Table => {
            let mut table = comfy_table::Table::new();
            table.load_style(UTF8_FULL);
            table.set_header(vec!["ID", "Plan", "Prio", "Due", "Status", "Title", "Tags"]);
            for t in tasks {
                table.add_row(vec![
                    t.id.to_string(),
                    t.plan_id.map(|p| p.to_string()).unwrap_or_default(),
                    t.priority.to_string(),
                    t.due.map(|d| d.to_string()).unwrap_or_default(),
                    t.status.to_string(),
                    t.title.clone(),
                    t.tags.join(", "),
                ]);
            }
            println!("{table}");
            println!("{} task(s)", tasks.len());
        }
    }
    Ok(())
}

pub fn dispatch(cmd: Commands, conn: &Connection, fmt: Format) -> Result<()> {
    match cmd {
        Commands::Add {
            title,
            plan,
            due,
            priority,
            tags,
            detail,
        } => task::add(conn, &title, plan, due, priority, tags, detail),
        Commands::List {
            plan,
            overdue,
            all,
            done,
        } => task::list(conn, plan, overdue, all, done, fmt),
        Commands::Show { id } => task::show(conn, id, fmt),
        Commands::Done { id } => task::done(conn, id),
        Commands::Rm { id } => task::rm(conn, id),
        Commands::Mod {
            id,
            title,
            due,
            priority,
            status,
            tags,
        } => task::modify(conn, id, title, due, priority, status, tags),
        Commands::Plan { cmd } => plan::run(conn, cmd, fmt),
        Commands::Decision { cmd } => decision::run(conn, cmd, fmt),
        Commands::Stats => stats::run(conn, fmt),
        // handled in lib::execute before the database is opened
        Commands::Init | Commands::Generate { .. } => Ok(()),
    }
}
