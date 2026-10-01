use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};

use crate::error::DpError;
use crate::models::{Adr, AdrStatus};

const COLS: &str =
    "id, number, title, context, decision, consequence, status, superseded_by, decided_at";

pub struct NewAdr<'a> {
    pub title: &'a str,
    pub context: &'a str,
    pub decision: &'a str,
    pub consequence: &'a str,
}

pub fn add(conn: &Connection, d: &NewAdr) -> Result<i64> {
    let next: i64 = conn.query_row("SELECT COALESCE(MAX(id), 0) + 1 FROM adrs", [], |r| {
        r.get(0)
    })?;
    conn.execute(
        "INSERT INTO adrs (number, title, context, decision, consequence)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            crate::models::adr_number(next),
            d.title,
            d.context,
            d.decision,
            d.consequence
        ],
    )?;
    Ok(conn.last_insert_rowid())
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

pub fn accept(conn: &Connection, id: i64) -> Result<()> {
    let d = get(conn, id)?;
    if let Some(by) = d.superseded_by {
        return Err(DpError::AlreadySuperseded(id, by).into());
    }
    conn.execute(
        "UPDATE adrs SET status = 'accepted', decided_at = datetime('now','localtime') WHERE id = ?1",
        params![id],
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
        number: row.get(1)?,
        title: row.get(2)?,
        context: row.get(3)?,
        decision: row.get(4)?,
        consequence: row.get(5)?,
        status: AdrStatus::from_label(&status).unwrap_or(AdrStatus::Proposed),
        superseded_by: row.get(7)?,
        decided_at: row.get(8)?,
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
        accept(&c, b).unwrap();
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
        accept(&c, b).unwrap();
        accept(&c, c2).unwrap();
        supersede(&c, a, b).unwrap();
        assert!(supersede(&c, a, c2).is_err()); // already superseded
        assert!(supersede(&c, b, b).is_err()); // self
    }
}
