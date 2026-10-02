-- v1 -> v2: derivation chain (ADR→plan→task) + ADR review fields
-- 运行前提: 库已是 v1(sql/v1.sql 建的)

-- 1) plans 获得 ADR 溯源
ALTER TABLE plans ADD COLUMN adr_id INTEGER REFERENCES adrs(id) ON DELETE SET NULL;

-- 2) adrs 获得评审字段(rationale 列 + rejected 状态需重建表以扩展 CHECK)
CREATE TABLE adrs_new (
    id            INTEGER PRIMARY KEY,
    number        TEXT NOT NULL UNIQUE,
    title         TEXT NOT NULL,
    context       TEXT NOT NULL,
    decision      TEXT NOT NULL,
    consequence   TEXT NOT NULL DEFAULT '',
    rationale     TEXT NOT NULL DEFAULT '',
    status        TEXT NOT NULL DEFAULT 'proposed'
                  CHECK (status IN ('proposed', 'accepted', 'rejected', 'superseded')),
    superseded_by INTEGER REFERENCES adrs(id),
    decided_at    TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);
INSERT INTO adrs_new (id, number, title, context, decision, consequence, rationale, status, superseded_by, decided_at, created_at)
    SELECT id, number, title, context, decision, consequence, '', status, superseded_by, decided_at, created_at FROM adrs;
DROP TABLE adrs;
ALTER TABLE adrs_new RENAME TO adrs;

-- 3) tasks 卸下 ADR 直连(先删索引,再删列——SQLite 要求)
DROP INDEX IF EXISTS idx_tasks_adr;
ALTER TABLE tasks DROP COLUMN adr_id;

-- 4) plans 的 ADR 溯源索引
CREATE INDEX IF NOT EXISTS idx_plans_adr ON plans(adr_id);
