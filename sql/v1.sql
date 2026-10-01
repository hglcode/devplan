CREATE TABLE IF NOT EXISTS plans (
    id           INTEGER PRIMARY KEY,
    title        TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    status       TEXT NOT NULL DEFAULT 'active'
                 CHECK (status IN ('active', 'done', 'archived')),
    priority     INTEGER NOT NULL DEFAULT 1
                 CHECK (priority BETWEEN 0 AND 3),
    start_date   TEXT,
    due_date     TEXT,
    created_at   TEXT NOT NULL DEFAULT (datetime('now','localtime')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE IF NOT EXISTS tasks (
    id           INTEGER PRIMARY KEY,
    adr_id       INTEGER REFERENCES adrs(id) ON DELETE SET NULL,
    plan_id      INTEGER REFERENCES plans(id) ON DELETE SET NULL,
    type         TEXT NOT NULL DEFAULT 'feature'
                 CHECK (type IN ('feature', 'bug', 'chore', 'refactor', 'docs')),
    title        TEXT NOT NULL,
    description  TEXT NOT NULL DEFAULT '',
    status       TEXT NOT NULL DEFAULT 'todo'
                 CHECK (status IN ('todo', 'active', 'blocked', 'done')),
    resolution   TEXT
                 CHECK (resolution IS NULL OR resolution IN ('done', 'abandoned', 'duplicate')),
    priority     INTEGER NOT NULL DEFAULT 1
                 CHECK (priority BETWEEN 0 AND 3),
    tags         TEXT NOT NULL DEFAULT '',
    time_spent   REAL NOT NULL DEFAULT 0,
    due_date     TEXT,
    done_at      TEXT,
    started_at   TEXT,
    created_at   TEXT NOT NULL DEFAULT (datetime('now','localtime')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE IF NOT EXISTS adrs (
    id            INTEGER PRIMARY KEY,
    number        TEXT NOT NULL UNIQUE,
    title         TEXT NOT NULL,
    context       TEXT NOT NULL,
    decision      TEXT NOT NULL,
    consequence   TEXT NOT NULL DEFAULT '',
    status        TEXT NOT NULL DEFAULT 'proposed'
                  CHECK (status IN ('proposed', 'accepted', 'superseded')),
    superseded_by INTEGER REFERENCES adrs(id),
    decided_at    TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);
-- 索引同你的设计,外加:
CREATE INDEX IF NOT EXISTS idx_tasks_resolution ON tasks(resolution);
CREATE INDEX IF NOT EXISTS idx_tasks_plan ON tasks(plan_id);
CREATE INDEX IF NOT EXISTS idx_tasks_adr ON tasks(adr_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status_priority ON tasks(status, priority DESC);
CREATE INDEX IF NOT EXISTS idx_tasks_due ON tasks(due_date);
CREATE INDEX IF NOT EXISTS idx_tasks_done_at ON tasks(done_at);

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
