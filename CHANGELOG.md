# Changelog

## [0.13.0] — 2026-10-04

### Added

- `dp task start` — dedicated verb for entering active (attention-head);
  first writer of `started_at` (idempotent; refuses from done)
- `status_note` (schema migration v7): `mod --status X --note "..."` — the
  reason for a transition; cleared on next transition (process-scoped),
  symmetric to `resolution_note` (permanent, verdict-scoped)

### Changed — BREAKING

- schema migration v8: `time_spent` dropped — the last dead column.
  Time tracking is another product's philosophy; schema now has zero
  dead columns (every column has a writer, a reader, semantics)

## [0.12.0] — 2026-10-04

### Changed — BREAKING

- schema migration v6: resolution `duplicate` → `duplicated` (past-participle
  consistency); new columns `resolution_note` and `duplicate_of`
- `dp task duplicate` gains `--of <ID>` (linkage to the surviving task)
  and `--note`; `done`/`abandon` gain `--note` — every resolution now
  carries its reason (mirrors `dp adr accept --rationale`, ADR-007)

### Migration notes

- databases upgrade automatically; back up `.dp/` before upgrading —
  the rebuild migration touches views, indexes and the updated_at trigger
  (the three-part sandwich; all restored)

## [0.11.1] — 2026-10-04

### Fixed

- `dp task list` default view now shows in-progress work: pending tasks
  (todo/active/blocked) ordered by attention (active first, then blocked,
  then todo) — previously active tasks were invisible in the default view

## [0.11.0] — 2026-10-04

### Changed — BREAKING

- task commands nest under `dp task`: add/list/show/mod/rm/done/abandon/
  duplicate now require the noun (`dp task list`) — three entities
  (task/plan/adr) are now symmetric in the command tree
- schema migration v5: `adrs.number` dropped (proven ≡ f(id)); ADR-NNN
  becomes render-time format

### Fixed

- plan show listed no tasks: stale `TaskView::Active` (semantic change
  from 0.6.x) — restored `TaskView::Pending`

## [0.10.0] — 2026-10-03

### Changed

- `updated_at` is now maintained by database triggers on tasks/plans
  (schema migration v4) — the invariant lives in storage, so any write
  path (app code, hand-run SQL, even FK ON DELETE SET NULL) leaves a
  timestamp fingerprint; repo-layer manual SETs retired
- ADR-006; `adrs` stays created_at-only (immutable-document semantics)

## [0.9.0] — 2026-10-03

### Added

- `dp rm` always asks for confirmation before deleting (summary + [y/N],
  default no) — uniform safety, no exemption tier; `--force` skips the
  prompt for scripts and tests
- tests guarding both paths: prompt-abort, force-bypass, and the
  no-exemption baseline itself

### Fixed

- user-facing wording: "Added/Deleted/Updated todo" → "task"
  (vocabulary residue from the 0.5.0 rename)

## [0.8.0] — 2026-10-03

### Fixed — CRITICAL

- migration v3 no longer clears `tasks.plan_id`: the plans table rebuild
  triggered `ON DELETE SET NULL`. Migrations now run with foreign keys
  disabled and a post-migration `foreign_key_check`. If you already ran
  0.7.0's migration, plan linkage may need manual restoration — skip
  from 0.6.x directly to 0.8.0.

### Changed — BREAKING

- plan status becomes container semantics: `open`/`archived` replaces
  `active`/`done`; `dp plan archive` replaces `dp plan done`;
  `archived` + partial progress is legitimate history (ADR-005)

### Added

- `migration_v3_preserves_plan_linkage` regression test (the exact
  failure mode that shipped in 0.7.0)

## [0.7.0] — 2026-10-02

### Changed — BREAKING

- schema migration v2 (the first real migration): databases from 0.6.x
  upgrade automatically on first run — back up `.dp/` before upgrading
- derivation chain corrected: `task.adr_id` removed, `plan.adr_id` added —
  tasks derive from plans, plans derive from ADRs (ADR→plan→task)

### Added

- `dp adr reject` — verdicts are revisitable; changing a verdict
  (accept↔reject) requires `--rationale`
- `--rationale` on accept/reject: review reasoning recorded with the decision
- `dp adr show` — full ADR view including rationale
- `dp plan add --adr <ID>` — link a plan to its decision origin
- ADR status `rejected`; `superseded` remains the only terminal state

## [0.6.1] — 2026-10-02

### Changed

- SQL extracted from `db.rs` into `sql/v1.sql` (`include_str!` — single source of truth, syntax highlighting, `sqlite3 .read` workflow); design doc `db.sql` retired
- `v_plan_progress` view now part of the schema itself (fresh databases get it automatically)

### Decided

- resolution semantics finalized: `duplicate` counts as output and progress (real effort from in-plan misjudgment); `abandoned` (unfinished) stays excluded — boundary is completion, not intent (ADR-003)

## [0.6.0] — 2026-10-02

### Changed — BREAKING

- `dp decision` renamed to `dp adr` (vocabulary unification: schema → repo → CLI)

### Added

- `--type` flag on add/mod, `--type` filter on list (feature/bug/chore/refactor/docs)
- `--start` flag on plan add (schedule window left edge)
- `dp abandon` / `dp duplicate` commands — resolution axis (abandoned excluded from output stats)
- `dp mod --plan` — reassign a task's plan
- Type/Resolution lines in `dp show`; Progress column in `dp plan list` (v_plan_progress view)
- CHANGELOG.md

### Fixed

- plan list ordering (active-first was inverted)
- plans.start_date column mismatch (started_at)
- ADR number column in decision list

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
