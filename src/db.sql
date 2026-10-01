-- ============================================================
-- 1. 项目计划 (Plans / Milestones)
-- 用于宏观追踪进度（如：v1.0发布、Q3重构、核心模块开发）
-- ============================================================
CREATE TABLE IF NOT EXISTS plans (
    id           INTEGER PRIMARY KEY,
    title        TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',   -- 简述目标
    status       TEXT NOT NULL DEFAULT 'active'
                 CHECK (status IN ('backlog', 'active', 'paused', 'completed', 'cancelled')),

    -- 进度追踪核心字段
    start_date   TEXT,                       -- 计划开始日期 (YYYY-MM-DD)
    due_date     TEXT,                       -- 计划截止日期

    created_at   TEXT NOT NULL DEFAULT (datetime('now','localtime')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE INDEX IF NOT EXISTS idx_plans_status ON plans(status);


-- ============================================================
-- 2. 项目任务清单 (Tasks) - 核心执行单元
-- 解决“从哪开始”和“工作报告”的问题
-- ============================================================
CREATE TABLE IF NOT EXISTS tasks (
    id           INTEGER PRIMARY KEY,

    -- 归属与来源追踪
    plan_id      INTEGER REFERENCES plans(id) ON DELETE SET NULL, -- 属于哪个宏观计划
    adr_id       INTEGER REFERENCES adrs(id) ON DELETE SET NULL, -- 是否由某个决策衍生

    -- 任务属性
    type         TEXT NOT NULL DEFAULT 'feature'
                 CHECK (type IN ('feature', 'bug', 'chore', 'refactor', 'docs')),
    title        TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',   -- Markdown 补充说明

    -- 状态与执行 (解决“从哪开始”)
    status       TEXT NOT NULL DEFAULT 'todo'
                 CHECK (status IN ('todo', 'active', 'done', 'blocked', 'closed')),
    priority     INTEGER NOT NULL DEFAULT 2
                 CHECK (priority BETWEEN 0 AND 3),
                 -- 0:Low, 1:Normal, 2:High, 3:Urgent (数字越大越紧急)

    -- 报告与复盘 (解决“工作报告”)
    tags         TEXT NOT NULL DEFAULT '[]', -- JSON数组: '["frontend","api"]'
    time_spent   REAL DEFAULT 0,             -- 预估或实际耗时(小时)，用于周报统计

    due_date     TEXT,
    done_at      TEXT,                       -- 实际完成时间 (用于统计产出)
    started_at   TEXT,                       -- 实际开始时间 (用于统计进度)
    created_at   TEXT NOT NULL DEFAULT (datetime('now','localtime')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

-- 核心查询索引 (为 Dashboard 和 报告 优化)
CREATE INDEX IF NOT EXISTS idx_tasks_plan ON tasks(plan_id);
CREATE INDEX IF NOT EXISTS idx_tasks_adr ON tasks(adr_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status_priority ON tasks(status, priority DESC);
CREATE INDEX IF NOT EXISTS idx_tasks_done_at ON tasks(done_at);
CREATE INDEX IF NOT EXISTS idx_tasks_started_at ON tasks(started_at);


-- ============================================================
-- 3. 项目决策记录 (ADR)
-- 记录“为什么这么做”，避免重复踩坑
-- ============================================================
CREATE TABLE IF NOT EXISTS adrs (
    id            INTEGER PRIMARY KEY,
    number        TEXT NOT NULL UNIQUE,      -- 决策编号，如 "ADR-001" (用TEXT方便补零)
    title         TEXT NOT NULL,
    context       TEXT NOT NULL,             -- 背景：遇到了什么问题？
    decision      TEXT NOT NULL,             -- 决定：我们决定怎么做？
    consequence   TEXT NOT NULL DEFAULT '',  -- 后果：带来什么好处/坏处/技术债？

    status        TEXT NOT NULL DEFAULT 'proposed'
                  CHECK (status IN ('proposed', 'accepted', 'deprecated', 'superseded')),
    superseded_by INTEGER REFERENCES adrs(id), -- 被哪个新决策替代了

    decided_at    TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE INDEX IF NOT EXISTS idx_adrs_status ON adrs(status);


-- 1. `v_active_tasks` —— 解决“该从哪里开始？”
CREATE VIEW IF NOT EXISTS v_active_tasks AS
SELECT
    t.id,
    t.title,
    t.type,
    t.priority,
    t.status,
    t.due_date,
    p.title AS plan_title,
    d.number AS adr_number
FROM tasks t
LEFT JOIN plans p ON t.plan_id = p.id
LEFT JOIN adrs d ON t.adr_id = d.id
WHERE t.status IN ('active', 'blocked')
ORDER BY
    t.status ASC,      -- active 优先于 blocked
    t.priority DESC,   -- 高优先级在前
    t.due_date ASC;    -- 快过期的在前

-- 2. `v_plan_progress` —— 解决“一眼看进度”
CREATE VIEW IF NOT EXISTS v_plan_progress AS
SELECT
    p.id,
    p.title,
    p.status,
    p.start_date,
    p.due_date,
    COUNT(t.id) AS total_tasks,
    SUM(CASE WHEN t.status = 'done' THEN 1 ELSE 0 END) AS done_tasks,
    CASE
        WHEN COUNT(t.id) > 0
        THEN ROUND(100.0 * SUM(CASE WHEN t.status = 'done' THEN 1 ELSE 0 END) / COUNT(t.id), 1)
        ELSE 0
    END AS progress_pct
FROM plans p
LEFT JOIN tasks t ON p.id = t.plan_id
GROUP BY p.id
ORDER BY p.due_date ASC;

-- 3. `v_recent_done` —— 解决“工作报告”
CREATE VIEW IF NOT EXISTS v_recent_done AS
SELECT
    id,
    title,
    type,
    time_spent,
    done_at,
    date(done_at) AS done_date,
    strftime('%Y-W%W', done_at) AS done_week  -- ISO周数，方便按周分组
FROM tasks
WHERE status = 'done'
ORDER BY done_at DESC;

-- 4. `v_decisions_index` —— ADR 快速检索
CREATE VIEW IF NOT EXISTS v_decisions_index AS
SELECT
    d.number,
    d.title,
    d.status,
    d.created_at,
    s.number AS superseded_by_number  -- 直接显示替代它的ADR编号，而非ID
FROM adrs d
LEFT JOIN adrs s ON d.superseded_by = s.id
WHERE d.status != 'deprecated'  -- 默认隐藏已废弃的，需要时再查原表
ORDER BY d.number DESC;
