## [0.5.2] — 2026-10-01

### Fixed

- plan list column index after start_date addition
- ADR number column rendering

## [0.5.1] — 2026-10-01

### Fixed

- plans.start_date column name mismatch (was `started_at`)
- docs vocabulary sync (README + man page for schema v2)

## [0.5.0] — 2026-10-01

### Changed — BREAKING

- schema v2: `todos` → `tasks`, `decisions` → `adrs` (clean break, no migration)
- task status vocabulary: `open`/`in_progress` → `todo`/`active`
- priority scale inverted: `low/0 normal/1 high/2 urgent/3` (higher = more urgent)
- `--detail` flag unified to `--description` (-d)

### Added

- resolution axis (done/abandoned/duplicate) — abandoned excluded from output stats
- task `type` column (feature/bug/chore/refactor/docs) with CHECK
- ADR numbering (ADR-001, ADR-002, …)
- `adr_id` linkage on tasks, `started_at`/`time_spent` columns

## [0.4.0] — 2026-09-30

### Changed — BREAKING

- `--all` renamed to `--active` (the name over-promised; semantics unchanged at the time)

## [0.3.0] — 2026-09-30

### Added

- `--format table|json|markdown` output for read commands
- `dp mod --detail` (later `--description`), full-field dirty-check with distinct no-op messages
- `dp generate man` (clap_mangen reference output)

## [0.2.0] — 2026-09-30

### Changed — BREAKING

- `done` table absorbed into `todos` (status='done' + done_at); schema migration collapse
