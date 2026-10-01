pub mod adr;
pub mod plan;
pub mod stats;
pub mod task;

use anyhow::Result;
use comfy_table::presets::UTF8_FULL;
use rusqlite::Connection;

use crate::cli::{Commands, Format};
use crate::commands::task::{AddArgs, ModifyArgs};
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
                    t.due_date.map_or_else(|| "-".into(), |d| d.to_string()),
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
                    t.due_date.map(|d| d.to_string()).unwrap_or_default(),
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
            due_date,
            priority,
            tags,
            description,
            task_type,
        } => task::add(
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
        Commands::List {
            plan,
            overdue,
            active,
            done,
            task_type,
        } => task::list(conn, plan, overdue, active, done, task_type, fmt),
        Commands::Show { id } => task::show(conn, id, fmt),
        Commands::Done { id } => task::done(conn, id),
        Commands::Rm { id } => task::rm(conn, id),
        Commands::Mod {
            id,
            title,
            due_date,
            priority,
            status,
            tags,
            description,
            task_type,
            plan,
        } => task::modify(
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
        Commands::Plan { cmd } => plan::run(conn, cmd, fmt),
        Commands::Adr { cmd } => adr::run(conn, cmd, fmt),
        Commands::Stats => stats::run(conn, fmt),
        // handled in lib::execute before the database is opened
        Commands::Init | Commands::Generate { .. } => Ok(()),
    }
}
