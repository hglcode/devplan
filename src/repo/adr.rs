use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params, types::ToSql};

use crate::error::DpError;
use crate::models::{Adr, AdrStatus};

const COLS: &str = "id, title, context, decision, consequence, rationale, status, superseded_by, decided_at, created_at";

pub struct NewAdr<'a> {
    pub title: &'a str,
    pub context: &'a str,
    pub decision: &'a str,
    pub consequence: &'a str,
}

#[derive(Debug, Default, Clone)]
pub struct AdrUpdate {
    pub title: Option<String>,
    pub context: Option<String>,
    pub decision: Option<String>,
    pub consequence: Option<String>,
}

pub fn add(conn: &Connection, d: &NewAdr) -> Result<i64> {
    conn.execute(
        "INSERT INTO adrs (title, context, decision, consequence)
         VALUES (?1, ?2, ?3, ?4)",
        params![d.title, d.context, d.decision, d.consequence],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Applies the non-None fields. Content only — verdicts and their rationale
/// are review history and move exclusively through accept/reject/supersede.
/// (adrs has no updated_at column; there is nothing to timestamp here.)
/// Returns false when there was nothing to change.
pub fn update(conn: &Connection, id: i64, u: &AdrUpdate) -> Result<bool> {
    let existing = get(conn, id)?;

    let mut sets: Vec<String> = Vec::new();
    let mut owned: Vec<Box<dyn ToSql>> = Vec::new();

    if let Some(title) = &u.title
        && *title != existing.title
    {
        sets.push("title = ?".into());
        owned.push(Box::new(title.clone()));
    }
    if let Some(context) = &u.context
        && *context != existing.context
    {
        sets.push("context = ?".into());
        owned.push(Box::new(context.clone()));
    }
    if let Some(decision) = &u.decision
        && *decision != existing.decision
    {
        sets.push("decision = ?".into());
        owned.push(Box::new(decision.clone()));
    }
    if let Some(consequence) = &u.consequence
        && *consequence != existing.consequence
    {
        sets.push("consequence = ?".into());
        owned.push(Box::new(consequence.clone()));
    }

    if sets.is_empty() {
        return Ok(false);
    }

    owned.push(Box::new(id));
    let sql = format!("UPDATE adrs SET {} WHERE id = ?", sets.join(", "));
    let args: Vec<&dyn ToSql> = owned.iter().map(std::convert::AsRef::as_ref).collect();
    conn.execute(&sql, args.as_slice())?;
    Ok(true)
}

pub fn get(conn: &Connection, id: i64) -> Result<Adr> {
    conn.query_row(
        &format!("SELECT {COLS} FROM adrs WHERE id = ?1"),
        params![id],
        map_row,
    )
    .optional()?
    .ok_or(DpError::AdrNotFound(id))
    .map_err(Into::into)
}

pub fn list(conn: &Connection) -> Result<Vec<Adr>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLS} FROM adrs ORDER BY (status = 'accepted') DESC, id DESC"
    ))?;
    let rows = stmt.query_map([], map_row)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// Accepts a decision, optionally recording review rationale.
/// Rejected decisions may be re-accepted (verdicts can be revisited).
pub fn accept(conn: &Connection, id: i64, rationale: Option<&str>) -> Result<()> {
    let d = get(conn, id)?;
    if let Some(by) = d.superseded_by {
        return Err(DpError::AlreadySuperseded(id, by).into());
    }
    // 翻案(rejected→accepted)必须带理由:verdict 变更需要留痕
    if d.status == AdrStatus::Rejected && rationale.is_none() {
        return Err(DpError::RationaleRequired(id).into());
    }
    if d.status == AdrStatus::Accepted && rationale.is_none() {
        return Ok(()); // 幂等:已是 accepted 且无新理由
    }
    conn.execute(
        "UPDATE adrs SET status = 'accepted', rationale = COALESCE(?1, rationale),
         decided_at = datetime('now','localtime') WHERE id = ?2",
        params![rationale, id],
    )?;
    Ok(())
}

/// Rejects a proposed decision with optional rationale.
/// Re-acceptance is allowed later; only superseded is terminal.
pub fn reject(conn: &Connection, id: i64, rationale: Option<&str>) -> Result<()> {
    let d = get(conn, id)?;
    if let Some(by) = d.superseded_by {
        return Err(DpError::AlreadySuperseded(id, by).into());
    }
    // 翻案(accepted→rejected)同理必带理由
    if d.status == AdrStatus::Accepted && rationale.is_none() {
        return Err(DpError::RationaleRequired(id).into());
    }
    if d.status == AdrStatus::Rejected && rationale.is_none() {
        return Ok(()); // 幂等
    }
    conn.execute(
        "UPDATE adrs SET status = 'rejected',
                rationale = COALESCE(?1, rationale),
                decided_at = datetime('now','localtime')
         WHERE id = ?2",
        params![rationale, id],
    )?;
    Ok(())
}

pub fn supersede(conn: &Connection, old_id: i64, new_id: i64) -> Result<()> {
    if old_id == new_id {
        return Err(DpError::SelfSupersede(old_id).into());
    }
    let old = get(conn, old_id)?;
    let new = get(conn, new_id)?;
    if let Some(prev) = old.superseded_by {
        return Err(DpError::AlreadySuperseded(old_id, prev).into());
    }
    if new.status != AdrStatus::Accepted {
        return Err(DpError::NotAccepted(new_id, old_id).into());
    }
    conn.execute(
        "UPDATE adrs SET status = 'superseded', superseded_by = ?1 WHERE id = ?2",
        params![new_id, old_id],
    )?;
    Ok(())
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<Adr> {
    let status: String = row.get(6)?;
    Ok(Adr {
        id: row.get(0)?,
        title: row.get(1)?,
        context: row.get(2)?,
        decision: row.get(3)?,
        consequence: row.get(4)?,
        rationale: row.get(5)?,
        status: AdrStatus::from_label(&status).unwrap_or(AdrStatus::Proposed),
        superseded_by: row.get(7)?,
        decided_at: row.get(8)?,
        created_at: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrate;

    fn conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrate(&c).unwrap();
        c
    }

    fn add_one(conn: &Connection, title: &str) -> i64 {
        add(
            conn,
            &NewAdr {
                title,
                context: "ctx",
                decision: "dec",
                consequence: "",
            },
        )
        .unwrap()
    }

    #[test]
    fn supersede_requires_accepted_replacement() {
        let c = conn();
        let a = add_one(&c, "a");
        let b = add_one(&c, "b");
        assert!(supersede(&c, a, b).is_err()); // b is still proposed
        accept(&c, b, None).unwrap();
        supersede(&c, a, b).unwrap();
        let old = get(&c, a).unwrap();
        assert_eq!(old.status, AdrStatus::Superseded);
        assert_eq!(old.superseded_by, Some(b));
    }

    #[test]
    fn cannot_supersede_twice_or_self() {
        let c = conn();
        let a = add_one(&c, "a");
        let b = add_one(&c, "b");
        let c2 = add_one(&c, "c");
        accept(&c, b, None).unwrap();
        accept(&c, c2, None).unwrap();
        supersede(&c, a, b).unwrap();
        assert!(supersede(&c, a, c2).is_err()); // already superseded
        assert!(supersede(&c, b, b).is_err()); // self
    }

    #[test]
    fn reject_then_reaccept_roundtrip() {
        let c = conn();
        let a = add_one(&c, "a");
        reject(&c, a, Some("not now")).unwrap();
        // 翻案无理由 → 拒绝:
        assert!(accept(&c, a, None).is_err());
        // 翻案带理由 → 通过且覆盖:
        accept(&c, a, Some("revisited — now it fits")).unwrap();
        let d = get(&c, a).unwrap();
        assert_eq!(d.status, AdrStatus::Accepted);
        assert_eq!(d.rationale, "revisited — now it fits");
    }

    #[test]
    fn adr_update_content_and_noop() {
        let c = conn();
        let id = add_one(&c, "T");
        let u = AdrUpdate {
            title: Some("T2".into()),
            context: Some("ctx2".into()),
            ..Default::default()
        };
        assert!(update(&c, id, &u).unwrap());
        let d = get(&c, id).unwrap();
        assert_eq!(d.title, "T2");
        assert_eq!(d.context, "ctx2");
        assert_eq!(d.decision, "dec"); // 未给,原样
        // 同值再跑 → false;零字段 → false
        assert!(!update(&c, id, &u).unwrap());
        assert!(!update(&c, id, &AdrUpdate::default()).unwrap());
    }

    #[test]
    fn adr_update_missing_id_is_clean_error() {
        let c = conn();
        let u = AdrUpdate {
            title: Some("x".into()),
            ..Default::default()
        };
        assert!(update(&c, 999, &u).is_err());
    }
}
