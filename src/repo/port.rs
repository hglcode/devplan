//! 全库快照:dp export 的数据源,dp import 的落库路径。
//!
//! 列值原样直通(不做枚举归一化):
//! - status/priority/type 等校验列以 String/INTEGER 原样读写,
//!   合法性交给 SQLite 的 CHECK 约束——import 时坏值在 INSERT 当场炸,
//!   不会静默入库;
//! - plans.status 的 DB 词表是 'active'/'done'/'archived',而领域枚举
//!   序列化成 "open"——归一化反而过不了 CHECK,直通是唯一正确路线;
//! - 由此换来 export → import → export 字节级一致。
use std::collections::HashMap;

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

use crate::models::{AdrRow, PlanRow, Snapshot, TaskRow, join_tags, split_tags};

// tasks 的 17 列,v10 定型(INSERT 与 SELECT 两边各数一遍——三明治血训)
const TASK_COLS: &str = "id, plan_id, type, title, description, status, status_note, \
     resolution, resolution_note, duplicate_of, priority, tags, due_date, \
     started_at, done_at, created_at, updated_at";

pub fn dump(conn: &Connection) -> Result<Snapshot> {
    let plans = {
        let mut stmt = conn.prepare(
            "SELECT id, adr_id, title, description, status, priority, \
                    start_date, due_date, created_at, updated_at \
             FROM plans ORDER BY id",
        )?;
        let rows = stmt.query_map([], map_plan_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    let adrs = {
        let mut stmt = conn.prepare(
            "SELECT id, title, context, decision, consequence, rationale, \
                    status, superseded_by, decided_at, created_at \
             FROM adrs ORDER BY id",
        )?;
        let rows = stmt.query_map([], map_adr_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    let mut tasks = {
        let mut stmt = conn.prepare(&format!("SELECT {TASK_COLS} FROM tasks ORDER BY id"))?;
        let rows = stmt.query_map([], map_task_row)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    // 依赖边:task_deps → 嵌进各任务的 deps 数组(与 --deps 的 CLI 心智一致)
    let mut deps: HashMap<i64, Vec<i64>> = HashMap::new();
    {
        let mut stmt =
            conn.prepare("SELECT task_id, depends_on FROM task_deps ORDER BY task_id, depends_on")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
        for row in rows {
            let (task, dep) = row?;
            deps.entry(task).or_default().push(dep);
        }
    }
    for t in &mut tasks {
        t.deps = deps.remove(&t.id).unwrap_or_default();
    }

    Ok(Snapshot {
        schema_version: crate::db::SCHEMA_VERSION,
        plans,
        tasks,
        adrs,
    })
}

/// 全量替换:清库重灌,原 id 原样写回(引用天然成立,零重映射)。
/// 单事务 + defer_foreign_keys——顺序不再敏感,悬空引用在 COMMIT 时暴露,
/// 此时事务回滚,原库分毫未动。
pub fn restore(conn: &Connection, snap: &Snapshot) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    // FK 延迟到 COMMIT:adrs 互引(superseded_by)、tasks 自引用(duplicate_of)
    // 都不怕插入/删除顺序;COMMIT 时统一校验,违例=整体回滚。
    tx.pragma_update(None, "defer_foreign_keys", "ON")?;

    // 清库(子表在前;defer 兜底,顺序其实已不敏感)
    tx.execute("DELETE FROM task_deps", [])?;
    tx.execute("DELETE FROM tasks", [])?;
    tx.execute("DELETE FROM plans", [])?;
    tx.execute("DELETE FROM adrs", [])?;

    for a in &snap.adrs {
        // 10 列:id(1) title(2) context(3) decision(4) consequence(5)
        //        rationale(6) status(7) superseded_by(8) decided_at(9) created_at(10)
        tx.execute(
            "INSERT INTO adrs (id, title, context, decision, consequence, \
                              rationale, status, superseded_by, decided_at, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                a.id,
                &a.title,
                &a.context,
                &a.decision,
                &a.consequence,
                &a.rationale,
                &a.status,
                &a.superseded_by,
                &a.decided_at,
                &a.created_at,
            ],
        )
        .with_context(|| format!("importing ADR #{}", a.id))?;
    }

    for p in &snap.plans {
        // 10 列:id(1) adr_id(2) title(3) description(4) status(5)
        //        priority(6) start_date(7) due_date(8) created_at(9) updated_at(10)
        tx.execute(
            "INSERT INTO plans (id, adr_id, title, description, status, priority, \
                               start_date, due_date, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                p.id,
                &p.adr_id,
                &p.title,
                &p.description,
                &p.status,
                p.priority,
                &p.start_date,
                &p.due_date,
                &p.created_at,
                &p.updated_at,
            ],
        )
        .with_context(|| format!("importing plan #{}", p.id))?;
    }

    for t in &snap.tasks {
        // 17 列,与 TASK_COLS 逐一对应;tags 数组 ↔ CSV 文本
        tx.execute(
            &format!(
                "INSERT INTO tasks ({TASK_COLS}) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, \
                         ?11, ?12, ?13, ?14, ?15, ?16, ?17)"
            ),
            params![
                t.id,
                &t.plan_id,
                &t.r#type,
                &t.title,
                &t.description,
                &t.status,
                &t.status_note,
                &t.resolution,
                &t.resolution_note,
                &t.duplicate_of,
                t.priority,
                join_tags(&t.tags),
                &t.due_date,
                &t.started_at,
                &t.done_at,
                &t.created_at,
                &t.updated_at,
            ],
        )
        .with_context(|| format!("importing task #{}", t.id))?;
    }

    // 依赖边最后灌(引用的任务行已就位;defer 之下顺序本就无感)
    for t in &snap.tasks {
        for &d in &t.deps {
            tx.execute(
                "INSERT INTO task_deps (task_id, depends_on) VALUES (?1, ?2)",
                params![t.id, d],
            )
            .with_context(|| format!("importing dependency {} -> {d}", t.id))?;
        }
    }

    Ok(tx.commit()?)
}

/// 当前库的三类计数(给 import 的确认提示看家底)。
pub fn counts(conn: &Connection) -> Result<(i64, i64, i64)> {
    let plans: i64 = conn.query_row("SELECT COUNT(*) FROM plans", [], |r| r.get(0))?;
    let tasks: i64 = conn.query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get(0))?;
    let adrs: i64 = conn.query_row("SELECT COUNT(*) FROM adrs", [], |r| r.get(0))?;
    Ok((plans, tasks, adrs))
}

fn map_plan_row(row: &rusqlite::Row) -> rusqlite::Result<PlanRow> {
    Ok(PlanRow {
        id: row.get(0)?,
        adr_id: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        status: row.get(4)?,
        priority: row.get(5)?,
        start_date: row.get(6)?,
        due_date: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn map_adr_row(row: &rusqlite::Row) -> rusqlite::Result<AdrRow> {
    Ok(AdrRow {
        id: row.get(0)?,
        title: row.get(1)?,
        context: row.get(2)?,
        decision: row.get(3)?,
        consequence: row.get(4)?,
        rationale: row.get(5)?,
        status: row.get(6)?,
        superseded_by: row.get(7)?,
        decided_at: row.get(8)?,
        created_at: row.get(9)?,
    })
}

fn map_task_row(row: &rusqlite::Row) -> rusqlite::Result<TaskRow> {
    Ok(TaskRow {
        id: row.get(0)?,
        plan_id: row.get(1)?,
        r#type: row.get(2)?,
        title: row.get(3)?,
        description: row.get(4)?,
        status: row.get(5)?,
        status_note: row.get(6)?,
        resolution: row.get(7)?,
        resolution_note: row.get(8)?,
        duplicate_of: row.get(9)?,
        priority: row.get(10)?,
        tags: split_tags(&row.get::<_, String>(11)?),
        due_date: row.get(12)?,
        started_at: row.get(13)?,
        done_at: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
        deps: Vec::new(), // 由 dump 统一填充
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrate;
    use crate::models::{Priority, TaskType};
    use crate::repo::task::{self, NewTask};
    use crate::repo::{adr, plan};
    use rusqlite::Connection;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrate(&c).unwrap();
        c
    }

    /// 覆盖各列形态的迷你库:plan + adr(accepted) + 两任务(依赖边、
    /// done 终态、tags 含空格)。
    fn populated() -> Connection {
        let c = conn();
        let p = plan::add(
            &c,
            &plan::NewPlan {
                title: "P".into(),
                adr_id: None,
                description: "pd".into(),
                start_date: None,
                due_date: None,
                priority: Priority::High,
            },
        )
        .unwrap();
        let a = adr::add(
            &c,
            &adr::NewAdr {
                title: "A",
                context: "ctx",
                decision: "dec",
                consequence: "",
            },
        )
        .unwrap();
        adr::accept(&c, a, None).unwrap();
        let t1 = task::add(
            &c,
            &NewTask {
                title: "t1",
                plan: Some(p),
                task_type: TaskType::Bug,
                due_date: None,
                priority: Priority::Normal,
                tags: vec!["x".into(), "y z".into()],
                description: "d1",
            },
        )
        .unwrap();
        let t2 = task::add(
            &c,
            &NewTask {
                title: "t2",
                plan: None,
                task_type: TaskType::Feature,
                due_date: None,
                priority: Priority::Urgent,
                tags: vec![],
                description: "",
            },
        )
        .unwrap();
        task::set_deps(&c, t2, &[t1]).unwrap();
        // t1 走到终态(顺带触发一次 updated_at trigger,验证快照存的是触发后的值)
        c.execute(
            "UPDATE tasks SET status='done', resolution='done', resolution_note='rn', \
             status_note='sn', done_at='2026-11-06 09:00:00', due_date='2026-11-05' \
             WHERE id = ?1",
            params![t1],
        )
        .unwrap();
        c
    }

    fn new_task(title: &str) -> NewTask<'_> {
        NewTask {
            title,
            plan: None,
            task_type: TaskType::Feature,
            due_date: None,
            priority: Priority::Normal,
            tags: vec![],
            description: "",
        }
    }

    #[test]
    fn export_import_roundtrip_is_byte_identical() {
        let c = populated();
        let snap1 = dump(&c).unwrap();
        let json1 = serde_json::to_string(&snap1).unwrap();

        // 快照之后库继续变化:多一个游离任务、依赖边被清空
        task::add(&c, &new_task("stray")).unwrap();
        task::set_deps(&c, 2, &[]).unwrap();

        restore(&c, &snap1).unwrap();
        let json2 = serde_json::to_string(&dump(&c).unwrap()).unwrap();

        // 字节级一致一并证明:游离任务没了(全量替换)、依赖边回来了、
        // restore 走 INSERT 不触发 updated_at trigger(触发则 updated_at
        // 变"now",字节必不等——trigger 绊线内嵌于此)
        assert_eq!(json1, json2);
    }

    #[test]
    fn restore_preserves_ids_and_continues_sequence() {
        let c = populated();
        let snap = dump(&c).unwrap();
        restore(&c, &snap).unwrap();
        // t1=1, t2=2 → 新任务从 max+1 接续(rowid 语义,无 AUTOINCREMENT)
        let next = task::add(&c, &new_task("next")).unwrap();
        assert_eq!(next, 3);
    }

    #[test]
    fn restore_rejects_dangling_references_and_keeps_db() {
        let c = populated();
        let before = serde_json::to_string(&dump(&c).unwrap()).unwrap();

        // 悬空 plan 引用
        let mut snap = dump(&c).unwrap();
        snap.tasks[0].plan_id = Some(9999);
        assert!(restore(&c, &snap).is_err());
        assert_eq!(before, serde_json::to_string(&dump(&c).unwrap()).unwrap());

        // 悬空依赖边
        let mut snap = dump(&c).unwrap();
        snap.tasks[0].deps = vec![9999];
        assert!(restore(&c, &snap).is_err());
        assert_eq!(before, serde_json::to_string(&dump(&c).unwrap()).unwrap());
    }

    #[test]
    fn restore_rejects_invalid_enum_via_check() {
        let c = populated();
        let before = serde_json::to_string(&dump(&c).unwrap()).unwrap();
        let mut snap = dump(&c).unwrap();
        snap.tasks[0].status = "bogus".into();
        // CHECK 在 INSERT 当场拒绝(不等到 COMMIT),原库无损
        assert!(restore(&c, &snap).is_err());
        assert_eq!(before, serde_json::to_string(&dump(&c).unwrap()).unwrap());
    }
}
