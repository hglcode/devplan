-- v4 -> v5: adr number demoted to render-time format of id
-- number has UNIQUE constraint — cannot DROP COLUMN, rebuild table instead

CREATE TABLE adrs_new (
    id            INTEGER PRIMARY KEY,
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
INSERT INTO adrs_new (id, title, context, decision, consequence, rationale, status, superseded_by, decided_at, created_at)
    SELECT id, title, context, decision, consequence, rationale, status, superseded_by, decided_at, created_at FROM adrs;
DROP TABLE adrs;
ALTER TABLE adrs_new RENAME TO adrs;

CREATE INDEX IF NOT EXISTS idx_adrs_status ON adrs(status);
