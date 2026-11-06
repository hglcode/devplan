-- v10: 依赖从单列(tasks.depends_on)升格为关联表 task_deps(多对多)
-- v9 的 depends_on 带 inline REFERENCES,SQLite 拒绝对 FK 子列 DROP COLUMN → 整表重建三明治

-- ① 视图下线(RENAME 期间全 schema 重解析,谁引用 tasks 谁炸)
DROP VIEW IF EXISTS v_plan_progress;

-- ② 关联表上岗 + 存量迁入(数据源在旧列里,必须在重建 tasks 之前)
CREATE TABLE task_deps (
    task_id    INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    depends_on INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    PRIMARY KEY (task_id, depends_on)
);
CREATE INDEX IF NOT EXISTS idx_task_deps_dep ON task_deps(depends_on);
INSERT INTO task_deps (task_id, depends_on)
    SELECT id, depends_on FROM tasks WHERE depends_on IS NOT NULL;

-- ③ 重建 tasks:v6 骨架 + v7 的 status_note;v8 已删的 time_spent 不复活,v9 的 depends_on 不带
CREATE TABLE tasks_new (
    id              INTEGER PRIMARY KEY,
    plan_id         INTEGER REFERENCES plans(id) ON DELETE SET NULL,
    type            TEXT NOT NULL DEFAULT 'feature'
                    CHECK (type IN ('feature', 'bug', 'chore', 'refactor', 'docs')),
    title           TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    status          TEXT NOT NULL DEFAULT 'todo'
                    CHECK (status IN ('todo', 'active', 'blocked', 'done')),
    status_note     TEXT NOT NULL DEFAULT '',
    resolution      TEXT
                    CHECK (resolution IS NULL OR resolution IN ('done', 'abandoned', 'duplicated')),
    resolution_note TEXT NOT NULL DEFAULT '',
    duplicate_of    INTEGER REFERENCES tasks(id),
    priority        INTEGER NOT NULL DEFAULT 1
                    CHECK (priority BETWEEN 0 AND 3),
    tags            TEXT NOT NULL DEFAULT '',
    due_date        TEXT,
    done_at         TEXT,
    started_at      TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now','localtime')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

-- 17 列对 17 列,INSERT 与 SELECT 两边各数一遍(三明治血训):
-- id(1) plan_id(2) type(3) title(4) description(5) status(6) status_note(7)
-- resolution(8) resolution_note(9) duplicate_of(10) priority(11) tags(12)
-- due_date(13) done_at(14) started_at(15) created_at(16) updated_at(17)
INSERT INTO tasks_new (id, plan_id, type, title, description, status, status_note,
                       resolution, resolution_note, duplicate_of, priority, tags,
                       due_date, done_at, started_at, created_at, updated_at)
SELECT id, plan_id, type, title, description, status, status_note,
       resolution, resolution_note, duplicate_of, priority, tags,
       due_date, done_at, started_at, created_at, updated_at
FROM tasks;
DROP TABLE tasks;
ALTER TABLE tasks_new RENAME TO tasks;

-- ④ 索引复活(DROP TABLE 连带吞掉)
CREATE INDEX IF NOT EXISTS idx_tasks_plan ON tasks(plan_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status_priority ON tasks(status, priority DESC);
CREATE INDEX IF NOT EXISTS idx_tasks_due ON tasks(due_date);
CREATE INDEX IF NOT EXISTS idx_tasks_done_at ON tasks(done_at);
CREATE INDEX IF NOT EXISTS idx_tasks_resolution ON tasks(resolution);
CREATE INDEX IF NOT EXISTS idx_tasks_duplicate_of ON tasks(duplicate_of);

-- ⑤ trigger 复活(v4 的不变量必须活过 v10;updated_at_trigger_fires_on_raw_update 是绊线)
CREATE TRIGGER IF NOT EXISTS trg_tasks_updated_at
AFTER UPDATE ON tasks
FOR EACH ROW
WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE tasks SET updated_at = datetime('now','localtime') WHERE id = NEW.id;
END;

-- ⑥ 视图复活
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
