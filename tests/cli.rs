use assert_cmd::Command;
use predicates::prelude::*;
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
        .args(["list"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("dp init"));
}

#[test]
fn rejects_bad_date() {
    let dir = TempDir::new().unwrap();
    dp(&dir)
        .args(["add", "x", "--due", "15/10/2026"])
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
            "add",
            "write oauth",
            "--priority",
            "high",
            "--tags",
            "auth,backend",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("#1"));
    dp(&dir).args(["add", "write docs"]).assert().success();

    dp(&dir)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("write oauth"));

    dp(&dir).args(["done", "1"]).assert().success();
    dp(&dir)
        .args(["list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("write oauth").not());
    dp(&dir)
        .args(["list", "--done"])
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
    cmd().arg("add").arg("t1").assert().success();
    for args in [["--format=json", "list"], ["list", "--format=json"]] {
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
