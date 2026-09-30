//! dp — project dev plan tracker.

pub mod cli;
pub mod commands;
pub mod db;
pub mod error;
pub mod models;
pub mod repo;

use anyhow::Result;

use crate::cli::GenerateTarget;

/// Parse argv and execute. Used by main().
pub fn run() -> Result<()> {
    execute(cli::parse())
}

/// Execute a parsed command. Init and completion generation run before any
/// database is opened.
pub fn execute(cli: cli::Cli) -> Result<()> {
    let cli::Cli { format, command } = cli;
    match command {
        cli::Commands::Init => return db::init(),
        cli::Commands::Generate { target } => {
            match target {
                GenerateTarget::Man => print_man()?,
                shell => {
                    let shell = match shell {
                        GenerateTarget::Bash => clap_complete::Shell::Bash,
                        GenerateTarget::Zsh => clap_complete::Shell::Zsh,
                        GenerateTarget::Fish => clap_complete::Shell::Fish,
                        GenerateTarget::Elvish => clap_complete::Shell::Elvish,
                        GenerateTarget::PowerShell => clap_complete::Shell::PowerShell,
                        GenerateTarget::Man => unreachable!(),
                    };
                    print_completions(shell);
                }
            }
            return Ok(());
        }
        _ => {}
    }
    let conn = db::open()?;
    commands::dispatch(command, &conn, format)
}
fn print_completions(shell: clap_complete::Shell) {
    let mut cmd = cli::build();
    let name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
}

fn print_man() -> Result<()> {
    let cmd = cli::build();
    let man = clap_mangen::Man::new(cmd);
    man.render(&mut std::io::stdout())?;
    Ok(())
}
