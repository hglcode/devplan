# dp — project dev plan tracker

A small Rust CLI to track **plans / tasks / decisions (ADRs)** for one project.
Data lives in `.dp/dp.db` (SQLite) next to your project root; `dp` discovers it by walking up from the current directory (like `git` finds `.git`).

## Build & install

```bash
cargo build --release
cp target/release/dp ~/.local/bin/    # or anywhere on $PATH
```

## Quick start

```bash
cd your-project
dp init                                # creates .dp/dp.db

# plans (milestones / epics)
dp plan add "Q4 refactor auth" --due 2026-12-31 -p high
dp plan list
dp plan show 1

# tasks
dp add "Implement OAuth login" --plan 1 --due 2026-10-15 -p high -t auth,backend
dp list
dp list --overdue
dp show 2
dp mod 2 --status active
dp rm 3

# ADRs (decision records)
dp adr add "Use SQLite" \
  --context "Need zero-config embedded storage" \
  --decision "rusqlite with bundled SQLite" \
  --consequence "Single-file db, easy to back up"
dp adr list
dp adr accept 1
dp adr supersede 1 4              # old decision 1 replaced by new 4

# stats
dp stats
```

## Tables

- `plans` — milestone / phase goal; `start_date` + `due_date` form its schedule window
- `tasks` — all work items; status: todo / active / blocked / done,
  plus a resolution axis for finished work: done / abandoned / duplicate
  (`dp done <id>` sets status='done' + done_at; undo with `dp mod <id> --status todo`)
- `adrs` — decision records numbered ADR-001, ADR-002, …;
  lifecycle: proposed → accepted → superseded

## Priority

`low`/`0`, `normal`/`1` (default), `high`/`2`, `urgent`/`3` — a higher number is more urgent.

## Data file

- One SQLite db per project at `.dp/dp.db`.
- Add `.dp/` to `.gitignore` if you don't want to commit it, or commit it if you do.
- Inspect directly: `sqlite3 .dp/dp.db`.

## Output formats

Read commands (`list`, `show`, `stats`, `plan list/show`, `decision list`) accept `--format`:

```bash
dp list --format=markdown     # GitHub-renderable table for PRs/issues
dp list --format=json | jq .  # machine-readable
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
