use anyhow::Result;
use comfy_table::{Table, presets::UTF8_FULL};
use rusqlite::Connection;

use crate::cli::{AdrCommands, Format};
use crate::repo;
use crate::repo::adr::NewAdr;

pub fn run(conn: &Connection, cmd: AdrCommands, fmt: Format) -> Result<()> {
    match cmd {
        AdrCommands::Add {
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
        AdrCommands::List => list(conn, fmt),
        AdrCommands::Accept { id } => {
            repo::adr::accept(conn, id)?;
            println!("Adr #{id} marked as accepted");
            Ok(())
        }
        AdrCommands::Supersede { old, new } => {
            repo::adr::supersede(conn, old, new)?;
            println!("Adr #{old} superseded by #{new}");
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
    let id = repo::adr::add(
        conn,
        &NewAdr {
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
    let adrs = repo::adr::list(conn)?;
    match fmt {
        Format::Json => println!("{}", serde_json::to_string_pretty(&adrs)?),
        Format::Markdown => {
            println!("| Number | ID | Status | Superseded by | Decided | Title |");
            println!("| ------ | -- | ------ | ------------- | ------- | ----- |");
            for d in &adrs {
                println!(
                    "| {} | {} | {} | {} | {} | {} |",
                    d.number,
                    d.id,
                    d.status,
                    d.superseded_by
                        .map_or_else(|| "-".into(), |s| s.to_string()),
                    d.decided_at.as_deref().unwrap_or("-"),
                    d.title.replace('|', "\\|"),
                );
            }
        }
        Format::Table => {
            let mut table = Table::new();
            table.load_style(UTF8_FULL);
            table.set_header(vec![
                "Number",
                "ID",
                "Status",
                "Superseded by",
                "Decided",
                "Title",
            ]);
            for d in &adrs {
                table.add_row(vec![
                    d.number.clone(),
                    d.id.to_string(),
                    d.status.to_string(),
                    d.superseded_by.map(|s| s.to_string()).unwrap_or_default(),
                    d.decided_at.clone().unwrap_or_else(|| "-".into()),
                    d.title.clone(),
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}
