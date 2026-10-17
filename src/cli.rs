use chrono::NaiveDate;
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};

use crate::models::{Priority, TaskStatus, TaskType};

/// Comma-separated prerequisite task ids, e.g. `--deps 27,26`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DepIds(Vec<i64>);

impl DepIds {
    pub fn into_ids(self) -> Vec<i64> {
        self.0
    }
}

impl std::str::FromStr for DepIds {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut ids: Vec<i64> = s
            .split(',')
            .map(str::trim) // "27, 26" is fine
            .filter(|p| !p.is_empty()) // "27," is fine
            .map(|p| {
                p.parse::<i64>()
                    .map_err(|e| format!("invalid task id '{p}': {e}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(&bad) = ids.iter().find(|&&i| i <= 0) {
            return Err(format!("task id must be positive, got '{bad}'"));
        }
        ids.sort_unstable();
        ids.dedup();
        Ok(Self(ids))
    }
}

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
    about = "project development plan, task and decision tracker"
)]
pub struct Cli {
    /// Output format: table, json, or markdown
    #[arg(
        long,
        alias = "fmt",
        value_enum,
        default_value = "table",
        global = true
    )]
    pub format: Format,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize a dp database in this project (.dp/dp.db)
    Init,

    /// ADR (architecture decision record) operations
    Adr {
        #[command(subcommand)]
        cmd: AdrCommands,
    },

    /// Plan (milestone) operations
    Plan {
        #[command(subcommand)]
        cmd: PlanCommands,
    },

    /// Task operations
    Task {
        #[command(subcommand)]
        cmd: TaskCommands,
    },

    /// Show project stats
    Stats,

    // Generate shell completion or man doc
    #[command(alias = "gen")]
    Generate {
        /// Target: a shell for completions, or "man" for a manual page
        #[arg(value_enum)]
        target: GenerateTarget,
    },
}

#[derive(Debug, Subcommand)]
pub enum TaskCommands {
    /// Add a new task
    #[command(alias = "new")]
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
        #[arg(short, long = "prio", alias = "priority", value_parser = parse_priority)]
        priority: Option<Priority>,
        /// Comma-separated tags
        #[arg(long, aliases = ["tag", "label", "labels"])]
        tags: Option<String>,
        /// Comma-separated prerequisite task ids (e.g. 27,26)
        #[arg(long = "deps", value_name = "IDS", aliases = ["dep", "depends", "depends-on"])]
        depends: Option<DepIds>,
        /// Long description
        #[arg(short, long = "desc", alias = "description")]
        description: Option<String>,
        /// Type: feature / bug / chore / refactor/docs
        #[arg(short, long = "type", value_parser = parse_task_type)]
        task_type: Option<TaskType>,
    },

    /// List tasks
    #[command(alias = "ls")]
    List {
        /// Filter by plan id
        #[arg(long)]
        plan: Option<i64>,
        /// Show only overdue tasks
        #[arg(long)]
        overdue: bool,
        /// Filter by status: todo / active / blocked / done (default: pending)
        #[arg(short, long, value_parser = parse_status)]
        status: Option<TaskStatus>,
        /// Filter by task type: feature / bug / chore / refactor / docs
        #[arg(short, long = "type", value_parser = parse_task_type)]
        task_type: Option<TaskType>,
        /// Limit output to the first N tasks (after ordering)
        #[arg(short = 'n', long, value_parser = clap::value_parser!(i64).range(1..))]
        limit: Option<i64>,
    },

    /// Show one task in description
    Show { id: i64 },

    /// Search tasks by text: title, description, status/resolution notes
    Search { text: String },

    /// Start working on a task: sets status active, records started_at
    Start { id: i64 },

    /// Mark a task as done
    Done {
        id: i64,
        #[arg(long)]
        note: Option<String>,
    },

    /// Abandon a task: done with resolution 'abandoned' (excluded from output stats)
    Abandon {
        id: i64,
        #[arg(long)]
        note: Option<String>,
    },

    /// Close a task as duplicate of another
    Duplicate {
        id: i64,
        #[arg(long)]
        note: Option<String>,
        #[arg(long)]
        of: Option<i64>,
    },

    /// Delete a task
    Rm {
        id: i64,
        #[arg(short, long)]
        force: bool,
    },

    /// Modify a task's fields
    #[command(aliases = ["up", "update", "edit", "modify"])]
    Mod {
        /// ID number
        id: i64,
        #[arg(long)]
        /// Title content
        title: Option<String>,
        /// Planned due date, YYYY-MM-DD (schedule window right edge)
        #[arg(long = "due", value_parser = parse_date)]
        due_date: Option<NaiveDate>,
        /// Priority: urgent, high, normal, low
        #[arg(short, long = "prio", alias = "priority", value_parser = parse_priority)]
        priority: Option<Priority>,
        /// Status: todo / active / blocked / done
        #[arg(short, long, value_parser = parse_status)]
        /// Note content
        status: Option<TaskStatus>,
        #[arg(long = "note")]
        status_note: Option<String>,
        /// Tags/Labels
        #[arg(long, aliases = ["tag", "label", "labels"])]
        tags: Option<String>,
        /// Comma-separated prerequisite task ids (e.g. 27,26)
        #[arg(long = "deps", value_name = "IDS", aliases = ["dep", "depends", "depends-on"])]
        depends: Option<DepIds>,
        /// Description content
        #[arg(short, long = "desc", alias = "description")]
        description: Option<String>,
        /// Type: feature / bug / chore / refactor / docs
        #[arg(short, long = "type", value_parser = parse_task_type)]
        task_type: Option<TaskType>,
        /// Reassign to another plan
        #[arg(long)]
        plan: Option<i64>,
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
    #[command(alias = "new")]
    Add {
        /// Title content
        title: String,
        /// ID number of ADR(architecture decision record)
        #[arg(long = "adr")]
        adr_id: Option<i64>,
        /// Planned start date, YYYY-MM-DD (schedule window left edge)
        #[arg(long = "start", value_parser = parse_date)]
        start_date: Option<NaiveDate>,
        /// Planned due date, YYYY-MM-DD (schedule window right edge)
        #[arg(long = "due", value_parser = parse_date)]
        due_date: Option<NaiveDate>,
        /// Priority: urgent, high, normal, low
        #[arg(short, long = "prio", alias = "priority", value_parser = parse_priority)]
        priority: Option<Priority>,
        /// Description content
        #[arg(short, long = "desc", alias = "description")]
        description: Option<String>,
    },
    #[command(alias = "ls")]
    List {
        /// Limit output to the first N plans (after ordering)
        #[arg(short = 'n', long, value_parser = clap::value_parser!(i64).range(1..))]
        limit: Option<i64>,
    },
    /// Show a plan with its pending tasks and done count
    Show { id: i64 },
    /// Mark a plan as Archived
    Archive { id: i64 },
}

#[derive(Debug, Subcommand)]
pub enum AdrCommands {
    #[command(alias = "new")]
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
    #[command(alias = "ls")]
    List {
        /// Limit output to the first N decisions(ADRs) (after ordering)
        #[arg(short = 'n', long, value_parser = clap::value_parser!(i64).range(1..))]
        limit: Option<i64>,
    },
    /// Mark a proposed (or rejected) decision as accepted
    Accept {
        /// ID number
        id: i64,
        /// Review rationale recorded with the acceptance
        #[arg(long)]
        rationale: Option<String>,
    },

    /// Reject a proposed decision (may be re-accepted later — verdicts can be revisited)
    Reject {
        /// ID number
        id: i64,
        /// Review rationale recorded with the rejection
        #[arg(long)]
        rationale: Option<String>,
    },

    /// Mark an old decision as superseded by an accepted newer one
    Supersede { old: i64, new: i64 },

    /// Show one ADR in full, including rationale
    Show { id: i64 },
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dep_ids_parses_csv() {
        let ids = |s: &str| s.parse::<DepIds>().unwrap().into_ids();
        assert_eq!(ids("27, 26,"), vec![26, 27]);
        assert_eq!(ids("26,26"), vec![26]);
        assert!(ids("").is_empty());
        assert!(ids(",").is_empty());
    }
    #[test]
    fn dep_ids_rejects_garbage() {
        for bad in ["0", "-1", "x", "1,x"] {
            assert!(bad.parse::<DepIds>().is_err(), "{bad}");
        }
    }
}
