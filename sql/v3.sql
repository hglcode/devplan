-- v2 -> v3: plan status becomes container semantics (open/archived)

-- 1) 先撤视图(避开 DROP/RENAME 期间的失效校验)
DROP VIEW IF EXISTS v_plan_progress;

-- 2) 重建 plans 表
CREATE TABLE plans_new (
    id           INTEGER PRIMARY KEY,
    adr_id       INTEGER REFERENCES adrs(id) ON DELETE SET NULL,
    title        TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    status       TEXT NOT NULL DEFAULT 'open'
                 CHECK (status IN ('open', 'archived')),
    priority     INTEGER NOT NULL DEFAULT 1
                 CHECK (priority BETWEEN 0 AND 3),
    start_date   TEXT,
    due_date     TEXT,
    created_at   TEXT NOT NULL DEFAULT (datetime('now','localtime')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);
INSERT INTO plans_new SELECT id, adr_id, title, description,
    CASE status WHEN 'done' THEN 'archived' ELSE 'open' END,
    priority, start_date, due_date, created_at, updated_at FROM plans;
DROP TABLE plans;
ALTER TABLE plans_new RENAME TO plans;

-- 3) 索引
CREATE INDEX IF NOT EXISTS idx_plans_adr ON plans(adr_id);

-- 4) 视图复活(新表就位后重建)
CREATE VIEW IF NOT EXISTS v_plan_progress AS
SELECT
    p.id, p.title, p.status, p.start_date, p.due_date,
    COUNT(t.id) AS total_tasks,
    SUM(CASE WHEN t.status = 'done' THEN 1 ELSE 0 END) AS done_tasks,
    CASE WHEN COUNT(t.id) > 0
         THEN ROUND(100.0 * SUM(CASE WHEN t.status = 'done' THEN 1 ELSE 0 END) / COUNT(t.id), 1)
         ELSE 0 END AS progress_pct
FROM plans p
LEFT JOIN tasks t ON p.id = t.plan_id
GROUP BY p.id;
