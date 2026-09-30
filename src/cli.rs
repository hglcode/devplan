use chrono::NaiveDate;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};

use crate::models::{Priority, TaskStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Format {
    Table,
    Json,
    Markdown,
}

#[derive(Parser)]
#[command(
    name = "dp",
    version,
    about = "Per-project plan / todo / done / decision tracker"
)]
pub struct Cli {
    /// Output format: table, json, or markdown
    #[arg(long, value_enum, default_value = "table", global = true)]
    pub format: Format,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize a dp database in this project (.dp/dp.db)
    Init,

    /// Add a new todo task
    Add {
        /// Task title
        title: String,
        /// Attach to an existing plan id
        #[arg(long)]
        plan: Option<i64>,
        /// Due date, YYYY-MM-DD
        #[arg(long, value_parser = parse_date)]
        due: Option<NaiveDate>,
        /// Priority: high / medium / low (or 1 / 2 / 3)
        #[arg(short, long, value_parser = parse_priority)]
        priority: Option<Priority>,
        /// Comma-separated tags
        #[arg(short, long)]
        tags: Option<String>,
        /// Long description
        #[arg(long)]
        detail: Option<String>,
    },

    /// List todos
    List {
        /// Filter by plan id
        #[arg(long)]
        plan: Option<i64>,
        /// Show only overdue tasks
        #[arg(long)]
        overdue: bool,
        /// Include in_progress and blocked (default: open only)
        #[arg(long)]
        all: bool,
        /// Show only completed tasks
        #[arg(long)]
        done: bool,
    },

    /// Show one todo in detail
    Show { id: i64 },

    /// Mark a todo as done
    Done { id: i64 },

    /// Delete a todo
    Rm { id: i64 },

    /// Modify a todo's fields
    Mod {
        id: i64,
        #[arg(long)]
        title: Option<String>,
        #[arg(long, value_parser = parse_date)]
        due: Option<NaiveDate>,
        #[arg(long, value_parser = parse_priority)]
        priority: Option<Priority>,
        /// open / in_progress / blocked / done
        #[arg(long, value_parser = parse_status)]
        status: Option<TaskStatus>,
        #[arg(long)]
        tags: Option<String>,
        #[arg(long)]
        detail: Option<String>,
    },

    /// Plan (milestone) operations
    Plan {
        #[command(subcommand)]
        cmd: PlanCommands,
    },

    /// Decision record (ADR) operations
    Decision {
        #[command(subcommand)]
        cmd: DecisionCommands,
    },

    /// Show project stats
    Stats,

    /// Generate shell completions, e.g. `dp generate bash > ~/.local/share/bash-completion/completions/dp`
    Generate {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Debug, Subcommand)]
pub enum PlanCommands {
    Add {
        title: String,
        #[arg(long, value_parser = parse_date)]
        due: Option<NaiveDate>,
        #[arg(short, long, value_parser = parse_priority)]
        priority: Option<Priority>,
        #[arg(long)]
        description: Option<String>,
    },
    List,
    /// Show a plan with its open todos and done count
    Show {
        id: i64,
    },
    /// Mark a plan as done
    Done {
        id: i64,
    },
}

#[derive(Debug, Subcommand)]
pub enum DecisionCommands {
    Add {
        title: String,
        /// Background / problem statement
        #[arg(long)]
        context: String,
        /// The decision made
        #[arg(long)]
        decision: String,
        /// Consequences / follow-ups
        #[arg(long)]
        consequence: Option<String>,
    },
    List,
    /// Mark a decision as accepted
    Accept {
        id: i64,
    },
    /// Mark an old decision as superseded by an accepted newer one
    Supersede {
        old: i64,
        new: i64,
    },
}

pub fn build() -> clap::Command {
    Cli::command()
}

pub fn parse() -> Cli {
    Cli::parse()
}

fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| format!("invalid date '{s}', expected YYYY-MM-DD"))
}

fn parse_priority(s: &str) -> Result<Priority, String> {
    match s.to_lowercase().as_str() {
        "high" | "1" => Ok(Priority::High),
        "medium" | "2" => Ok(Priority::Medium),
        "low" | "3" => Ok(Priority::Low),
        other => Err(format!(
            "priority must be high/medium/low or 1/2/3, got '{other}'"
        )),
    }
}

fn parse_status(s: &str) -> Result<TaskStatus, String> {
    TaskStatus::from_label(s)
        .ok_or_else(|| format!("status must be open/in_progress/blocked/done, got '{s}'"))
}
