use anyhow::Result;
use comfy_table::{Table, presets::UTF8_FULL};
use rusqlite::Connection;

use crate::cli::{DecisionCommands, Format};
use crate::repo;
use crate::repo::decision::NewDecision;

pub fn run(conn: &Connection, cmd: DecisionCommands, fmt: Format) -> Result<()> {
    match cmd {
        DecisionCommands::Add {
            title,
            context,
            decision,
            consequence,
        } => add(
            conn,
            &title,
            &context,
            &decision,
            consequence.as_deref().unwrap_or(""),
        ),
        DecisionCommands::List => list(conn, fmt),
        DecisionCommands::Accept { id } => {
            repo::decision::accept(conn, id)?;
            println!("Decision #{id} marked as accepted");
            Ok(())
        }
        DecisionCommands::Supersede { old, new } => {
            repo::decision::supersede(conn, old, new)?;
            println!("Decision #{old} superseded by #{new}");
            Ok(())
        }
    }
}

fn add(
    conn: &Connection,
    title: &str,
    context: &str,
    decision: &str,
    consequence: &str,
) -> Result<()> {
    let id = repo::decision::add(
        conn,
        &NewDecision {
            title,
            context,
            decision,
            consequence,
        },
    )?;
    println!("Added decision #{id}: {title}");
    Ok(())
}

fn list(conn: &Connection, fmt: Format) -> Result<()> {
    let decisions = repo::decision::list(conn)?;
    match fmt {
        Format::Json => println!("{}", serde_json::to_string_pretty(&decisions)?),
        Format::Markdown => {
            println!("| ID | Status | Superseded by | Decided | Title |");
            println!("| -- | ------ | ------------- | ------- | ----- |");
            for d in &decisions {
                println!(
                    "| {} | {} | {} | {} | {} |",
                    d.id,
                    d.status,
                    d.superseded_by
                        .map_or_else(|| "-".into(), |s| s.to_string()),
                    d.decided_at,
                    d.title.replace('|', "\\|"),
                );
            }
        }
        Format::Table => {
            let mut table = Table::new();
            table.load_style(UTF8_FULL);
            table.set_header(vec!["ID", "Status", "Superseded by", "Decided", "Title"]);
            for d in &decisions {
                table.add_row(vec![
                    d.id.to_string(),
                    d.status.to_string(),
                    d.superseded_by.map(|s| s.to_string()).unwrap_or_default(),
                    d.decided_at.clone(),
                    d.title.clone(),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}
