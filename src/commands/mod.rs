pub mod decision;
pub mod plan;
pub mod stats;
pub mod task;

use anyhow::Result;
use rusqlite::Connection;

use crate::cli::Commands;

pub fn dispatch(cmd: Commands, conn: &Connection) -> Result<()> {
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
        } => task::list(conn, plan, overdue, all, done),
        Commands::Show { id } => task::show(conn, id),
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
        Commands::Plan { cmd } => plan::run(conn, cmd),
        Commands::Decision { cmd } => decision::run(conn, cmd),
        Commands::Stats => stats::run(conn),
        // handled in lib::execute before the database is opened
        Commands::Init | Commands::Generate { .. } => Ok(()),
    }
}
