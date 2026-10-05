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
        AdrCommands::List { limit } => list(conn, limit, fmt),
        AdrCommands::Accept { id, rationale } => {
            repo::adr::accept(conn, id, rationale.as_deref())?;
            println!("ADR #{id} marked as accepted");
            Ok(())
        }
        AdrCommands::Reject { id, rationale } => {
            repo::adr::reject(conn, id, rationale.as_deref())?;
            println!("ADR #{id} rejected");
            Ok(())
        }
        AdrCommands::Supersede { old, new } => {
            repo::adr::supersede(conn, old, new)?;
            println!("Adr #{old} superseded by #{new}");
            Ok(())
        }
        AdrCommands::Show { id } => show(conn, id, fmt),
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

fn list(conn: &Connection, limit: Option<i64>, fmt: Format) -> Result<()> {
    let adrs = repo::adr::list(conn)?;
    let adrs = if let Some(n) = limit {
        let mut t = adrs;
        t.truncate(n as usize);
        t
    } else {
        adrs
    };
    match fmt {
        Format::Json => println!("{}", serde_json::to_string_pretty(&adrs)?),
        Format::Markdown => {
            println!("| ID | Status | Superseded by | Decided | Title |");
            println!("| -- | ------ | ------------- | ------- | ----- |");
            for d in &adrs {
                println!(
                    "| {} | {} | {} | {} | {} |",
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
            table.set_header(vec!["ID", "Status", "Superseded by", "Decided", "Title"]);
            for d in &adrs {
                table.add_row(vec![
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

fn show(conn: &Connection, id: i64, fmt: Format) -> Result<()> {
    let d = repo::adr::get(conn, id)?;
    if fmt == Format::Json {
        println!("{}", serde_json::to_string_pretty(&d)?);
        return Ok(());
    }
    println!("  ID:          {}", d.id);
    println!("  Title:       {}", d.title);
    println!("  Status:      {}", d.status);
    println!("  Context:     {}", d.context);
    println!("  Decision:    {}", d.decision);
    if !d.consequence.is_empty() {
        println!("  Consequence: {}", d.consequence);
    }
    if !d.rationale.is_empty() {
        println!("  Rationale:   {}", d.rationale); // ★ 它的舞台
    }
    if let Some(by) = d.superseded_by {
        println!("  Superseded by: ADR-{:03}", by);
    }
    if let Some(t) = &d.decided_at {
        println!("  Decided at:  {t}");
    }
    println!("  Created at:  {}", d.created_at);
    Ok(())
}
