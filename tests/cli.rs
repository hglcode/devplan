use assert_cmd::Command;
use predicates::prelude::*;
use std::ops::Not;
use tempfile::TempDir;

fn dp(dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("dp").unwrap();
    cmd.current_dir(dir.path());
    cmd
}

#[test]
fn init_is_idempotent() {
    let dir = TempDir::new().unwrap();
    dp(&dir).arg("init").assert().success();
    dp(&dir)
        .arg("init")
        .assert()
        .success()
        .stdout(predicates::str::contains("already exists"));
}

#[test]
fn fails_without_init() {
    let dir = TempDir::new().unwrap();
    dp(&dir)
        .args(["task", "list"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("dp init"));
}

#[test]
fn rejects_bad_date() {
    let dir = TempDir::new().unwrap();
    dp(&dir)
        .args(["task", "add", "x", "--due", "15/10/2026"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("invalid date"));
}

#[test]
fn add_list_done_flow() {
    let dir = TempDir::new().unwrap();
    dp(&dir).arg("init").assert().success();

    dp(&dir)
        .args([
            "task",
            "add",
            "write oauth",
            "--priority",
            "high",
            "--tag",
            "auth,backend",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("#1"));
    dp(&dir)
        .args(["task", "add", "write docs"])
        .assert()
        .success();

    dp(&dir)
        .args(["task", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("write oauth"));

    dp(&dir).args(["task", "done", "1"]).assert().success();
    dp(&dir)
        .args(["task", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("write oauth").not());
    dp(&dir)
        .args(["task", "list", "--status", "done"])
        .assert()
        .success()
        .stdout(predicates::str::contains("write oauth"));

    dp(&dir)
        .args(["stats"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Done total:"));
}

#[test]
fn adr_lifecycle() {
    let dir = TempDir::new().unwrap();
    dp(&dir).arg("init").assert().success();
    dp(&dir)
        .args([
            "adr",
            "add",
            "Use SQLite",
            "--context",
            "embedded",
            "--decision",
            "rusqlite",
        ])
        .assert()
        .success();
    dp(&dir)
        .args([
            "adr",
            "add",
            "Use Postgres",
            "--context",
            "managed",
            "--decision",
            "sqlx",
        ])
        .assert()
        .success();

    // replacing with a still-proposed decision is rejected (ADR discipline)
    dp(&dir)
        .args(["adr", "supersede", "1", "2"])
        .assert()
        .failure();
    dp(&dir).args(["adr", "accept", "2"]).assert().success();
    dp(&dir)
        .args(["adr", "supersede", "1", "2"])
        .assert()
        .success();
    dp(&dir)
        .args(["adr", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("superseded"));
}

#[test]
fn generate_completions() {
    let dir = TempDir::new().unwrap();
    dp(&dir)
        .args(["generate", "bash"])
        .assert()
        .success()
        .stdout(predicates::str::contains("dp"));
}

#[test]
fn format_json_is_parseable() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = || {
        let mut c = Command::cargo_bin("dp").unwrap();
        c.current_dir(&dir);
        c
    };
    cmd().arg("init").assert().success();
    cmd().arg("task").arg("add").arg("t1").assert().success();
    for args in [
        ["--format=json", "task", "list"].as_slice(),
        ["task", "list", "--format=json"].as_slice(),
    ] {
        let out = cmd().args(args).assert().success();
        let stdout = String::from_utf8(out.get_output().stdout.clone()).unwrap();
        serde_json::from_str::<serde_json::Value>(&stdout).expect("stdout must be pure JSON");
    }
}

#[test]
fn generate_man() {
    Command::cargo_bin("dp")
        .unwrap()
        .arg("generate")
        .arg("man")
        .assert()
        .success()
        .stdout(predicates::str::contains(".TH dp"));
}

#[test]
fn rm_with_history_prompts_and_aborts() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = || {
        let mut c = Command::cargo_bin("dp").unwrap();
        c.current_dir(&dir);
        c
    };
    cmd().arg("init").assert().success();
    cmd()
        .arg("task")
        .arg("add")
        .arg("survivor")
        .assert()
        .success();
    // 制造历史:done 过(有 done_at)→ has_history = true
    cmd().arg("task").arg("done").arg("1").assert().success();

    let out = cmd()
        .arg("task")
        .arg("rm")
        .arg("1")
        .write_stdin("n\n") // 拒绝
        .assert()
        .success()
        .stdout(predicates::str::contains("Delete permanently?"));
    let _ = out;

    // 判据:任务仍活着
    cmd()
        .arg("task")
        .arg("list")
        .arg("--status")
        .arg("done")
        .assert()
        .success()
        .stdout(predicates::str::contains("survivor"));
}

#[test]
fn rm_force_bypasses_prompt() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = || {
        let mut c = Command::cargo_bin("dp").unwrap();
        c.current_dir(&dir);
        c
    };
    cmd().arg("init").assert().success();
    cmd()
        .arg("task")
        .arg("add")
        .arg("victim")
        .assert()
        .success();
    cmd().arg("task").arg("done").arg("1").assert().success(); // 制造历史

    cmd()
        .arg("task")
        .arg("rm")
        .arg("1")
        .arg("--force")
        .assert()
        .success()
        .stdout(predicates::str::contains("Deleted task #1"));

    // 判据:真删了——done 列表里没有 victim
    cmd()
        .arg("task")
        .arg("list")
        .arg("--status")
        .arg("done")
        .assert()
        .success()
        .stdout(predicates::str::contains("victim").not());
}

#[test]
fn rm_always_prompts_even_without_history() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = || {
        let mut c = Command::cargo_bin("dp").unwrap();
        c.current_dir(&dir);
        c
    };
    cmd().arg("init").assert().success();
    cmd().arg("task").arg("add").arg("fresh").assert().success();
    // 无任何操作——直接 rm,喂 n
    cmd()
        .arg("task")
        .arg("rm")
        .arg("1")
        .write_stdin("n\n")
        .assert()
        .success()
        .stdout(predicates::str::contains("Delete permanently?"));
    // 底线:无历史的也活着
    cmd()
        .arg("task")
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("fresh"));
}

#[test]
fn list_default_is_pending_attention_ordered() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = || {
        let mut c = Command::cargo_bin("dp").unwrap();
        c.current_dir(&dir);
        c
    };
    cmd().arg("init").assert().success();
    cmd()
        .arg("task")
        .arg("add")
        .arg("queued")
        .assert()
        .success(); // #1 todo
    cmd().arg("task").arg("add").arg("stuck").assert().success(); // #2 → blocked
    cmd()
        .arg("task")
        .arg("add")
        .arg("working")
        .assert()
        .success(); // #3 → active
    cmd()
        .arg("task")
        .arg("mod")
        .arg("2")
        .arg("--status")
        .arg("blocked")
        .assert()
        .success();
    cmd()
        .arg("task")
        .arg("mod")
        .arg("3")
        .arg("--status")
        .arg("active")
        .assert()
        .success();

    let out = cmd()
        .arg("task")
        .arg("list")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(out).unwrap();
    // 三态都在(Pending 默认):
    assert!(stdout.contains("queued"));
    assert!(stdout.contains("stuck"));
    assert!(stdout.contains("working"));
    // 完整顺序链:active < blocked < todo
    let pos_working = stdout.find("working").unwrap();
    let pos_stuck = stdout.find("stuck").unwrap();
    let pos_queued = stdout.find("queued").unwrap();
    assert!(pos_working < pos_stuck, "active must sort before blocked");
    assert!(pos_stuck < pos_queued, "blocked must sort before todo");
}

#[test]
fn task_search_hits_title_and_note() {
    let dir = tempfile::tempdir().unwrap();
    let cmd = || {
        let mut c = Command::cargo_bin("dp").unwrap();
        c.current_dir(&dir);
        c
    };
    cmd().arg("init").assert().success();
    cmd()
        .arg("task")
        .arg("add")
        .arg("implement oauth")
        .assert()
        .success();
    cmd()
        .arg("task")
        .arg("add")
        .arg("unrelated")
        .assert()
        .success();
    cmd()
        .arg("task")
        .arg("mod")
        .arg("2")
        .arg("--status")
        .arg("blocked")
        .arg("--note")
        .arg("waiting on oauth provider")
        .assert()
        .success();
    cmd()
        .arg("task")
        .arg("add")
        .arg("completely different topic")
        .assert()
        .success();
    // title 命中 + note 命中,unrelated 不在:
    let out = cmd()
        .arg("task")
        .arg("search")
        .arg("oauth")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8(out).unwrap();
    assert!(s.contains("implement oauth"));
    assert!(s.contains("unrelated")); // note 命中
    assert!(s.contains("completely different topic").not()); // ★ 真正的排除判据
}
