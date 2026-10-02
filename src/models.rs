use chrono::NaiveDate;
use serde::Serialize;

// ─────────────────────────────────────────────
// 优先级:0-3,数字越大越紧急(与 DB 刻度一致)
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Low,
    #[default]
    Normal, // 默认 1,DB DEFAULT 1 与之对齐
    High,
    Urgent,
}

impl Priority {
    pub fn as_int(self) -> i64 {
        match self {
            Self::Low => 0,
            Self::Normal => 1,
            Self::High => 2,
            Self::Urgent => 3,
        }
    }
    pub fn from_int(v: i64) -> Option<Self> {
        match v {
            0 => Some(Self::Low),
            1 => Some(Self::Normal),
            2 => Some(Self::High),
            3 => Some(Self::Urgent),
            _ => None,
        }
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Urgent => "urgent",
        })
    }
}

// ─────────────────────────────────────────────
// 任务状态:流转轴(todo → active → done,blocked 为侧态)
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    #[default]
    Todo,
    Active, // 正在处理(原 in_progress)
    Blocked,
    Done,
}

impl TaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::Active => "active",
            Self::Blocked => "blocked",
            Self::Done => "done",
        }
    }
    pub fn from_label(s: &str) -> Option<Self> {
        match s {
            "todo" => Some(Self::Todo),
            "active" => Some(Self::Active),
            "blocked" => Some(Self::Blocked),
            "done" => Some(Self::Done),
            _ => None,
        }
    }
    /// 终态判定:进入 resolution 语义的节点
    pub fn is_done(self) -> bool {
        self == Self::Done
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─────────────────────────────────────────────
// 终态性质:第二轴,仅 status=done 时有意义
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Resolution {
    Done,      // 正常完成
    Abandoned, // 主动放弃(不算产出)
    Duplicate, // 与他任务重复
}

impl Resolution {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Abandoned => "abandoned",
            Self::Duplicate => "duplicate",
        }
    }
    pub fn from_label(s: &str) -> Option<Self> {
        match s {
            "done" => Some(Self::Done),
            "abandoned" => Some(Self::Abandoned),
            "duplicate" => Some(Self::Duplicate),
            _ => None,
        }
    }
    /// 是否计入产出统计(周报/今日完成)
    pub fn counts_as_output(self) -> bool {
        self != Self::Abandoned
    }
}

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─────────────────────────────────────────────
// 任务类型(GitHub Projects 同构)
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    #[default]
    Feature,
    Bug,
    Chore,
    Refactor,
    Docs,
}

impl TaskType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Feature => "feature",
            Self::Bug => "bug",
            Self::Chore => "chore",
            Self::Refactor => "refactor",
            Self::Docs => "docs",
        }
    }
    pub fn from_label(s: &str) -> Option<Self> {
        match s {
            "feature" => Some(Self::Feature),
            "bug" => Some(Self::Bug),
            "chore" => Some(Self::Chore),
            "refactor" => Some(Self::Refactor),
            "docs" => Some(Self::Docs),
            _ => None,
        }
    }
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─────────────────────────────────────────────
// 计划状态(收敛为三态)
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanStatus {
    Active,
    Done,
    Archived,
}

impl PlanStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Done => "done",
            Self::Archived => "archived",
        }
    }
    pub fn from_label(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "done" => Some(Self::Done),
            "archived" => Some(Self::Archived),
            _ => None,
        }
    }
}

impl std::fmt::Display for PlanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─────────────────────────────────────────────
// ADR 状态(三态;number 的格式化责任在应用层)
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AdrStatus {
    Proposed,
    Accepted,
    Rejected,
    Superseded,
}

impl AdrStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
            Self::Superseded => "superseded",
        }
    }
    pub fn from_label(s: &str) -> Option<Self> {
        match s {
            "proposed" => Some(Self::Proposed),
            "accepted" => Some(Self::Accepted),
            "rejected" => Some(Self::Rejected),
            "superseded" => Some(Self::Superseded),
            _ => None,
        }
    }
}

impl std::fmt::Display for AdrStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─────────────────────────────────────────────
// 领域对象(与新表列一一对齐)
// ─────────────────────────────────────────────
#[derive(Debug, Clone, Serialize)]
pub struct Task {
    pub id: i64,
    pub plan_id: Option<i64>,
    pub r#type: TaskType,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub resolution: Option<Resolution>,
    pub priority: Priority,
    pub tags: Vec<String>,
    pub time_spent: f64,
    pub due_date: Option<NaiveDate>,
    pub started_at: Option<String>,
    pub done_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Plan {
    pub id: i64,
    pub adr_id: Option<i64>,
    pub title: String,
    pub description: String,
    pub status: PlanStatus,
    pub priority: Priority,
    pub due_date: Option<NaiveDate>,
    pub start_date: Option<NaiveDate>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Adr {
    pub id: i64,
    pub number: String, // "ADR-001"
    pub title: String,
    pub context: String,
    pub decision: String,
    pub consequence: String,
    pub rationale: String,
    pub status: AdrStatus,
    pub superseded_by: Option<i64>,
    pub decided_at: Option<String>,
    pub created_at: String,
}

pub fn split_tags(s: &str) -> Vec<String> {
    s.split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn join_tags(tags: &[String]) -> String {
    tags.join(",")
}

/// 生成下一个 ADR 编号。next_id 是将要分配的自增 id。
pub fn adr_number(next_id: i64) -> String {
    format!("ADR-{next_id:03}")
}
