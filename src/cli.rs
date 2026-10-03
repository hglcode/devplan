use chrono::NaiveDate;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};

use crate::models::{Priority, TaskStatus, TaskType};

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
    about = "project plan, task and decision tracker"
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
        #[arg(long = "due", value_parser = parse_date)]
        due_date: Option<NaiveDate>,
        /// Priority: low/normal/high/urgent or 0/1/2/3
        #[arg(short, long, value_parser = parse_priority)]
        priority: Option<Priority>,
        /// Comma-separated tags
        #[arg(short, long)]
        tags: Option<String>,
        /// Long description
        #[arg(long)]
        description: Option<String>,
        /// Type: feature / bug / chore / refactor/docs
        #[arg(long = "type", value_parser = parse_task_type)]
        task_type: Option<TaskType>,
    },

    /// List tasks
    List {
        /// Filter by plan id
        #[arg(long)]
        plan: Option<i64>,
        /// Show only overdue tasks
        #[arg(long)]
        overdue: bool,
        /// Only tasks with status active
        #[arg(long)]
        active: bool,
        /// Show only completed tasks
        #[arg(long)]
        done: bool,
        /// Filter by task type: feature / bug / chore / refactor / docs
        #[arg(long = "type", value_parser = parse_task_type)]
        task_type: Option<TaskType>,
    },

    /// Show one task in description
    Show { id: i64 },

    /// Mark a task as done
    Done { id: i64 },

    /// Abandon a task: done with resolution 'abandoned' (excluded from output stats)
    Abandon { id: i64 },

    /// Close a task as duplicate of another
    Duplicate { id: i64 },

    /// Delete a task
    Rm { id: i64 },

    /// Modify a task's fields
    Mod {
        id: i64,
        #[arg(long)]
        title: Option<String>,
        #[arg(long = "due", value_parser = parse_date)]
        due_date: Option<NaiveDate>,
        #[arg(short, long, value_parser = parse_priority)]
        priority: Option<Priority>,
        /// todo / active / blocked / done
        #[arg(long, value_parser = parse_status)]
        status: Option<TaskStatus>,
        #[arg(short, long)]
        tags: Option<String>,
        #[arg(short, long)]
        description: Option<String>,
        /// Type: feature / bug / chore / refactor / docs
        #[arg(long = "type", value_parser = parse_task_type)]
        task_type: Option<TaskType>,
        /// Reassign to another plan
        #[arg(long)]
        plan: Option<i64>,
    },

    /// Plan (milestone) operations
    Plan {
        #[command(subcommand)]
        cmd: PlanCommands,
    },

    /// ADR (architecture decision record) operations
    Adr {
        #[command(subcommand)]
        cmd: AdrCommands,
    },

    /// Show project stats
    Stats,

    Generate {
        /// Target: a shell for completions, or "man" for a manual page
        #[arg(value_enum)]
        target: GenerateTarget,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum GenerateTarget {
    Bash,
    Zsh,
    Fish,
    Elvish,
    PowerShell,
    Man,
}

#[derive(Debug, Subcommand)]
pub enum PlanCommands {
    Add {
        title: String,
        #[arg(long = "adr")]
        adr_id: Option<i64>,
        /// Planned start date, YYYY-MM-DD (schedule window left edge)
        #[arg(long = "start", value_parser = parse_date)]
        start_date: Option<NaiveDate>,
        #[arg(long = "due", value_parser = parse_date)]
        due_date: Option<NaiveDate>,
        #[arg(short, long, value_parser = parse_priority)]
        priority: Option<Priority>,
        #[arg(short, long)]
        description: Option<String>,
    },
    List,
    /// Show a plan with its pending tasks and done count
    Show {
        id: i64,
    },
    /// Mark a plan as Archived
    Archive {
        id: i64,
    },
}

#[derive(Debug, Subcommand)]
pub enum AdrCommands {
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
    /// Mark a proposed (or rejected) decision as accepted
    Accept {
        id: i64,
        /// Review rationale recorded with the acceptance
        #[arg(long)]
        rationale: Option<String>,
    },

    /// Reject a proposed decision (may be re-accepted later — verdicts can be revisited)
    Reject {
        id: i64,
        /// Review rationale recorded with the rejection
        #[arg(long)]
        rationale: Option<String>,
    },

    /// Mark an old decision as superseded by an accepted newer one
    Supersede {
        old: i64,
        new: i64,
    },

    /// Show one ADR in full, including rationale
    Show {
        id: i64,
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
        "low" | "0" => Ok(Priority::Low),
        "normal" | "1" => Ok(Priority::Normal),
        "high" | "2" => Ok(Priority::High),
        "urgent" | "3" => Ok(Priority::Urgent),
        other => Err(format!(
            "priority must be low/normal/high/urgent or 0/1/2/3, got '{other}'"
        )),
    }
}
fn parse_status(s: &str) -> Result<TaskStatus, String> {
    TaskStatus::from_label(s)
        .ok_or_else(|| format!("status must be todo/active/blocked/done, got '{s}'"))
}

fn parse_task_type(s: &str) -> Result<TaskType, String> {
    TaskType::from_label(s)
        .ok_or_else(|| format!("type must be feature/bug/chore/refactor/docs, got '{s}'"))
}
