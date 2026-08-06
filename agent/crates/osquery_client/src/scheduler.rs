use crate::client::OsqueryClient;
use crate::diff;
use crate::types::{
    ColumnEntry, OsqueryResult, OsqueryResultRow, OsqueryRow, ResultAction, ScheduledQuery,
};
use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
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

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM scheduled_queries", [], |row| {
            row.get(0)
        })?;

        if count == 0 {
            let now = chrono::Utc::now().timestamp();
            let defaults = vec![
                (
                    "running_processes",
                    "SELECT pid, name, path, cmdline, uid FROM processes;",
                    30,
                    0,
                ),
                (
                    "listening_ports",
                    "SELECT pid, port, protocol, address FROM listening_ports;",
                    30,
                    0,
                ),
                (
                    "users",
                    "SELECT username, uid, gid, shell, directory FROM users;",
                    300,
                    0,
                ),
            ];
            for (name, query, interval, snapshot) in defaults {
                conn.execute(
                    "INSERT INTO scheduled_queries (name, query, interval_secs, snapshot, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![name, query, interval, snapshot, now],
                )?;
            }
            tracing::info!("Seeded default scheduled queries into empty database");
        }

        Ok(Self { conn })
    }

    pub fn load_queries(&self) -> Result<Vec<ScheduledQuery>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, query, interval_secs, snapshot FROM scheduled_queries")?;
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

        tx.commit()?;
        Ok(())
    }

    /// Run the scheduler. Each scheduled query gets its own task with a persistent
    /// OsqueryClient connection that reconnects on error.
    ///
    /// Queries are loaded *before* entering the async context so that the
    /// rusqlite::Connection is never held across an await point.
    pub async fn run(
        self,
        tx: mpsc::Sender<OsqueryResult>,
        socket_path: PathBuf,
        agent_uuid: String,
    ) {
        // Load queries synchronously before dropping self (and its Connection).
        let queries = match self.load_queries() {
            Ok(q) => q,
            Err(e) => {
                tracing::error!("Failed to load scheduled queries from SQLite: {}", e);
                return;
            }
        };
        // Drop self here — Connection is no longer held.
        drop(self);

        if queries.is_empty() {
            tracing::warn!("No scheduled queries found in SQLite — nothing to run.");
            return;
        }

        tracing::info!("Starting {} scheduled query task(s)", queries.len());

        for query in queries {
            let tx = tx.clone();
            let socket_path = socket_path.clone();
            let agent_uuid = agent_uuid.clone();

            tokio::spawn(async move {
                // Connect once per query task; reconnect on error inside the loop.
                let mut client = loop {
                    match OsqueryClient::connect(&socket_path).await {
                        Ok(c) => break c,
                        Err(e) => {
                            tracing::warn!(
                                "[{}] Cannot connect to osquery yet ({}), retrying in 5s...",
                                query.name,
                                e
                            );
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        }
                    }
                };

                let mut previous_rows: Vec<OsqueryRow> = Vec::new();
                let mut first_run = true;
                let mut interval =
                    tokio::time::interval(std::time::Duration::from_secs(query.interval_secs));

                loop {
                    interval.tick().await;

                    let response = match client.query(&query.query).await {
                        Ok(res) => res,
                        Err(e) => {
                            tracing::warn!("[{}] Query error: {}", query.name, e);
                            // Client will reconnect internally on next call.
                            continue;
                        }
                    };

                    if response.status.code != 0 {
                        tracing::warn!(
                            "[{}] osquery error (code {}): {}",
                            query.name,
                            response.status.code,
                            response.status.message
                        );
                        continue;
                    }

                    let current_rows = response.rows;
                    tracing::debug!("[{}] Got {} rows", query.name, current_rows.len());

                    if query.snapshot {
                        // Snapshot mode: emit all rows every tick.
                        let result = Self::build_result(
                            &query.name,
                            &agent_uuid,
                            current_rows,
                            ResultAction::Snapshot,
                        );
                        if tx.send(result).await.is_err() {
                            tracing::info!(
                                "[{}] Result channel closed, stopping task.",
                                query.name
                            );
                            break;
                        }
                    } else {
                        // Differential mode.
                        if first_run {
                            // First run: emit a full snapshot as the baseline.
                            let result = Self::build_result(
                                &query.name,
                                &agent_uuid,
                                current_rows.clone(),
                                ResultAction::Snapshot,
                            );
                            if tx.send(result).await.is_err() {
                                tracing::info!(
                                    "[{}] Result channel closed, stopping task.",
                                    query.name
                                );
                                break;
                            }
                            first_run = false;
                        } else {
                            let (added, removed) =
                                diff::compute_diff(&previous_rows, &current_rows);

                            if !added.is_empty() {
                                let res = Self::build_result(
                                    &query.name,
                                    &agent_uuid,
                                    added,
                                    ResultAction::Added,
                                );
                                if tx.send(res).await.is_err() {
                                    tracing::info!(
                                        "[{}] Result channel closed, stopping task.",
                                        query.name
                                    );
                                    break;
                                }
                            }
                            if !removed.is_empty() {
                                let res = Self::build_result(
                                    &query.name,
                                    &agent_uuid,
                                    removed,
                                    ResultAction::Removed,
                                );
                                if tx.send(res).await.is_err() {
                                    tracing::info!(
                                        "[{}] Result channel closed, stopping task.",
                                        query.name
                                    );
                                    break;
                                }
                            }
                        }
                        previous_rows = current_rows;
                    }
                }
            });
        }
    }

    fn build_result(
        query_name: &str,
        agent_uuid: &str,
        rows: Vec<OsqueryRow>,
        action: ResultAction,
    ) -> OsqueryResult {
        let mut result_rows = Vec::with_capacity(rows.len());
        for row in rows {
            let mut columns = Vec::with_capacity(row.len());
            for (k, v) in row {
                columns.push(ColumnEntry { name: k, value: v });
            }
            result_rows.push(OsqueryResultRow { columns });
        }

        OsqueryResult {
            query_name: query_name.to_string(),
            agent_uuid: agent_uuid.to_string(),
            timestamp_ns: Utc::now().timestamp_nanos_opt().unwrap_or(0),
            rows: result_rows,
            action,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upsert_and_load_queries() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("queries.db");
        let mut scheduler = QueryScheduler::new(&db_path).unwrap();

        let q1 = ScheduledQuery {
            name: "test_query".to_string(),
            query: "SELECT 1".to_string(),
            interval_secs: 60,
            snapshot: false,
        };

        scheduler.upsert_queries(std::slice::from_ref(&q1)).unwrap();

        let loaded = scheduler.load_queries().unwrap();
        assert_eq!(loaded.len(), 4);
        let q_loaded = loaded.iter().find(|q| q.name == "test_query").unwrap();
        assert_eq!(q_loaded.name, "test_query");
        assert_eq!(q_loaded.query, "SELECT 1");
        assert_eq!(q_loaded.interval_secs, 60);
        assert!(!q_loaded.snapshot);

        let q2 = ScheduledQuery {
            name: "test_query".to_string(),
            query: "SELECT 2".to_string(),
            interval_secs: 120,
            snapshot: true,
        };
        scheduler.upsert_queries(&[q2]).unwrap();

        let loaded = scheduler.load_queries().unwrap();
        assert_eq!(loaded.len(), 4);
        let q_loaded2 = loaded.iter().find(|q| q.name == "test_query").unwrap();
        assert_eq!(q_loaded2.query, "SELECT 2");
        assert_eq!(q_loaded2.interval_secs, 120);
        assert!(q_loaded2.snapshot);
    }
}
