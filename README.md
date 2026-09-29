# dp — per-project dev plan tracker

A small Rust CLI to track **plans / todos / done / decisions** for one project.
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

# todos
dp add "Implement OAuth login" --plan 1 --due 2026-10-15 -p high -t auth,backend
dp list
dp list --overdue
dp show 2
dp mod 2 --status in_progress
dp done 2                              # moves row: todos -> done
dp rm 3

# decisions (ADR)
dp decision add "Use SQLite" \
  --context "Need zero-config embedded storage" \
  --decision "rusqlite with bundled SQLite" \
  --consequence "Single-file db, easy to back up"
dp decision list
dp decision accept 1
dp decision supersede 1 4              # old decision 1 replaced by new 4

# stats
dp stats
```

## Tables

- `plans` — milestone / phase goal
- `todos` — all tasks; status: open / in_progress / blocked / done
  (`dp done <id>` sets status='done' + done_at; undo with `dp mod <id> --status open`)
- `decisions` — ADR lifecycle: proposed → accepted → superseded

## Priority

`high`/`1`, `medium`/`2` (default), `low`/`3`.

## Data file

- One SQLite db per project at `.dp/dp.db`.
- Add `.dp/` to `.gitignore` if you don't want to commit it, or commit it if you do.
- Inspect directly: `sqlite3 .dp/dp.db`.

## Shell completions

```bash
dp generate bash | zsh | fish | elvish | powershell
```
