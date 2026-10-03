-- v5 -> v6: resolution as a verdict system
-- tense fix + linkage + note

-- resolution CHECK 'duplicate' -> 'duplicated' + two new columns
-- (CHECK lives in the table definition — full rebuild required;
--  ADD COLUMN first is unnecessary when rebuilding anyway)

DROP VIEW IF EXISTS v_plan_progress;

CREATE TABLE tasks_new (
    id              INTEGER PRIMARY KEY,
    plan_id         INTEGER REFERENCES plans(id) ON DELETE SET NULL,
    type            TEXT NOT NULL DEFAULT 'feature'
                    CHECK (type IN ('feature', 'bug', 'chore', 'refactor', 'docs')),
    title           TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL DEFAULT 'todo'
                    CHECK (status IN ('todo', 'active', 'blocked', 'done')),
    resolution      TEXT
                    CHECK (resolution IS NULL OR resolution IN ('done', 'abandoned', 'duplicated')),
    resolution_note TEXT NOT NULL DEFAULT '',
    duplicate_of    INTEGER REFERENCES tasks(id),
    priority        INTEGER NOT NULL DEFAULT 1
                    CHECK (priority BETWEEN 0 AND 3),
    tags            TEXT NOT NULL DEFAULT '',
    time_spent      REAL NOT NULL DEFAULT 0,
    due_date        TEXT,
    done_at         TEXT,
    started_at      TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now','localtime')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

-- 17 columns, 17 values — count them before touching:
-- id(1) plan_id(2) type(3) title(4) description(5) status(6) resolution(7)
-- resolution_note(8) duplicate_of(9) priority(10) tags(11) time_spent(12)
-- due_date(13) done_at(14) started_at(15) created_at(16) updated_at(17)
INSERT INTO tasks_new (id, plan_id, type, title, description, status,
                       resolution, resolution_note, duplicate_of, priority,
                       tags, time_spent, due_date, done_at, started_at,
                       created_at, updated_at)
SELECT id, plan_id, type, title, description, status,
       CASE resolution WHEN 'duplicate' THEN 'duplicated' ELSE resolution END,
       '', NULL,
       priority, tags, time_spent, due_date, done_at, started_at,
       created_at, updated_at
FROM tasks;

DROP TABLE tasks;
ALTER TABLE tasks_new RENAME TO tasks;

-- indexes (rebuild drops them):
CREATE INDEX IF NOT EXISTS idx_tasks_plan ON tasks(plan_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status_priority ON tasks(status, priority DESC);
CREATE INDEX IF NOT EXISTS idx_tasks_due ON tasks(due_date);
CREATE INDEX IF NOT EXISTS idx_tasks_done_at ON tasks(done_at);
CREATE INDEX IF NOT EXISTS idx_tasks_resolution ON tasks(resolution);
CREATE INDEX IF NOT EXISTS idx_tasks_duplicate_of ON tasks(duplicate_of);

-- updated_at trigger (tasks rebuild drops it — v4's invariant must survive v6):
CREATE TRIGGER IF NOT EXISTS trg_tasks_updated_at
AFTER UPDATE ON tasks
FOR EACH ROW
WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE tasks SET updated_at = datetime('now','localtime') WHERE id = NEW.id;
END;

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
