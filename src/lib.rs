//! dp — per-project dev plan tracker.

pub mod cli;
pub mod commands;
pub mod db;
pub mod error;
pub mod models;
pub mod repo;

use anyhow::Result;

/// Parse argv and execute. Used by main().
pub fn run() -> Result<()> {
    execute(cli::parse())
}

/// Execute a parsed command. Init and completion generation run before any
/// database is opened.
pub fn execute(cmd: cli::Commands) -> Result<()> {
    match cmd {
        cli::Commands::Init => return db::init(),
        cli::Commands::Generate { shell } => {
            return {
                print_completions(shell);
                Ok(())
            };
        }
        _ => {}
    }
    let conn = db::open()?;
    commands::dispatch(cmd, &conn)
}

fn print_completions(shell: clap_complete::Shell) {
    let mut cmd = cli::build();
    let name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
}
