# dp — project dev plan tracker

A small Rust CLI to track **plans / tasks / decisions (ADRs)** for one project.
Data lives in `.dp/dp.db` (SQLite) next to your project root; `dp` discovers it by walking up from the current directory (like `git` finds `.git`).

## Build & install

```bash
# build from source
cargo build --release
cp target/release/dp ~/.local/bin/    # or anywhere on $PATH

# install with cargo
cargo install devplan
```

## Quick start

```bash
cd your-project
dp init                                # creates .dp/dp.db

# plans (milestones / epics) — containers of work
dp plan add "Q4 refactor auth" --due 2026-12-31 -p high
dp plan list                           # per-plan progress (done/total)
dp plan show 1
dp plan archive 1                      # close the container; partial progress is fine

# tasks — the work items
dp task add "Implement OAuth login" --plan 1 --due 2026-10-15 -p high -t auth,backend
dp task list                           # pending view: active first, then blocked, then todo
dp task list --overdue
dp task list --type bug                # filter by type
dp task show 2
dp task mod 2 --status active          # start working
dp task done 2                         # finish (resolution: done)
dp task abandon 3                      # drop unfinished (excluded from output stats)
dp task rm 4                           # always asks; --force skips

# ADRs (decision records)
dp adr add "Use SQLite" \
  --context "Need zero-config embedded storage" \
  --decision "rusqlite with bundled SQLite" \
  --consequence "Single-file db, easy to back up"
dp adr list
dp adr accept 1
dp adr reject 2 --rationale "not now"  # verdicts are revisitable; changing one requires --rationale
dp adr supersede 1 4                   # old decision 1 replaced by new 4

# stats
dp stats
```

## Tables

- `plans` — milestone / phase **container**; status open / archived;
  `start_date` + `due_date` form its schedule window;
  progress is derived from its tasks (archived + 60% is legitimate history)
- `tasks` — all work items; status axis: todo / active / blocked / done,
  resolution axis (finished work): done / abandoned / duplicate —
  abandoned is excluded from output stats, duplicate counts (the work was real)
- `adrs` — decision records shown as ADR-001, ADR-002, … (rendered from id);
  lifecycle: proposed → accepted ⇄ rejected → superseded (final)

## Priority

`low`/`0`, `normal`/`1` (default), `high`/`2`, `urgent`/`3` — a higher number is more urgent.

## Data file

- One SQLite db per project at `.dp/dp.db`.
- Add `.dp/` to `.gitignore` if you don't want to commit it, or commit it if you do.
- Inspect directly: `sqlite3 .dp/dp.db`.
- Schema lives in `sql/` (v1–v5, compiled in via `include_str!`);
  databases upgrade automatically on first run of a newer dp — back up `.dp/` before major upgrades.

## Output formats

Read commands (`task list/show`, `stats`, `plan list/show`, `adr list`) accept `--format`:

```bash
dp task list --format=markdown     # GitHub-renderable table for PRs/issues
dp task list --format=json | jq .  # machine-readable
dp stats --format=json
```

Default is `table`. Empty results still emit headers/schema (e.g. `[]` for json).

Escape hatch for anything not covered: query the SQLite file directly

```bash
sqlite3 .dp/dp.db -json "SELECT id,title,status FROM tasks"
```

## Shell completions

```bash
dp generate bash | zsh | fish | elvish | powershell | man
```

See `man dp` (or `man ./man/dp.1` from the repo) for the full manual.
