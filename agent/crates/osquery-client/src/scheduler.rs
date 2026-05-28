use crate::types::{OsqueryResult, ScheduledQuery};
use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use tokio::sync::mpsc;

pub struct QueryScheduler {
    conn: Connection,
}

impl QueryScheduler {
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS scheduled_queries (
                name TEXT PRIMARY KEY,
                query TEXT NOT NULL,
                interval_secs INTEGER NOT NULL,
                snapshot INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    pub fn load_queries(&self) -> Result<Vec<ScheduledQuery>> {
        let mut stmt = self.conn.prepare("SELECT name, query, interval_secs, snapshot FROM scheduled_queries")?;
        let query_iter = stmt.query_map([], |row| {
            let snapshot: i32 = row.get(3)?;
            Ok(ScheduledQuery {
                name: row.get(0)?,
                query: row.get(1)?,
                interval_secs: row.get(2)?,
                snapshot: snapshot != 0,
            })
        })?;

        let mut queries = Vec::new();
        for query in query_iter {
            queries.push(query?);
        }
        Ok(queries)
    }

    pub fn upsert_queries(&mut self, queries: &[ScheduledQuery]) -> Result<()> {
        let tx = self.conn.transaction()?;
        
        let now = chrono::Utc::now().timestamp();
        
        for query in queries {
            tx.execute(
                "INSERT INTO scheduled_queries (name, query, interval_secs, snapshot, updated_at) 
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(name) DO UPDATE SET 
                 query=excluded.query, interval_secs=excluded.interval_secs, snapshot=excluded.snapshot, updated_at=excluded.updated_at",
                rusqlite::params![
                    query.name,
                    query.query,
                    query.interval_secs,
                    if query.snapshot { 1 } else { 0 },
                    now,
                ],
            )?;
        }
        
        // Remove old queries not in this update (if we consider this update to be the full state)
        // Note: For now, we just upsert. If full replacement is needed, we'd delete missing ones.

        tx.commit()?;
        Ok(())
    }

    pub async fn run(self, _tx: mpsc::Sender<OsqueryResult>) {
        // Implement the actual loop here.
        // It will spawn tasks for each query, using OsqueryClient::query, and tracking diffs.
        // Left as stub for now until client is implemented.
    }
}
