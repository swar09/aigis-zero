use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Local SQLite-backed buffer for JSON-encoded AgentEvent objects.
/// Used when the fleet server is unreachable.
#[derive(Clone)]
pub struct EventBuffer {
    conn: Arc<Mutex<Connection>>,
    max_events: u64,
}

impl EventBuffer {
    /// Open or create the SQLite database at the given path.
    /// Creates the event_buffer table if it doesn't exist.
    pub fn new(db_path: &Path, max_events: u64) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Enable WAL mode for better concurrency
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS event_buffer (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                payload    JSON    NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            max_events,
        })
    }

    /// Store a JSON-encoded AgentEvent.
    /// The string comes from serde_json::to_string().
    pub async fn push(&self, event_json: String) -> Result<()> {
        let conn = self.conn.clone();
        let max_events = self.max_events;
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            let now = chrono::Utc::now().timestamp();
            conn.execute(
                "INSERT INTO event_buffer (payload, created_at) VALUES (?1, ?2)",
                rusqlite::params![event_json, now],
            )?;

            // Bounded cap eviction: if over max_events, delete the oldest
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM event_buffer", [], |row| row.get(0))?;
            if count > max_events as i64 {
                let to_delete = count - max_events as i64;
                conn.execute(
                    "DELETE FROM event_buffer WHERE id IN (SELECT id FROM event_buffer ORDER BY id ASC LIMIT ?1)",
                    [to_delete],
                )?;
            }

            Ok::<(), anyhow::Error>(())
        })
        .await??;
        Ok(())
    }

    /// Read and remove the oldest `batch_size` events.
    /// Returns raw JSON strings that can be parsed back.
    pub async fn drain(&self, batch_size: usize) -> Result<Vec<String>> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = conn.lock().unwrap();
            let tx = conn.transaction()?;

            let mut stmt =
                tx.prepare("SELECT id, payload FROM event_buffer ORDER BY id ASC LIMIT ?1")?;

            let mut events = Vec::new();
            let mut ids = Vec::new();

            let rows = stmt.query_map([batch_size], |row| {
                let id: i64 = row.get(0)?;
                let payload: String = row.get(1)?;
                Ok((id, payload))
            })?;

            for row in rows {
                let (id, payload) = row?;
                ids.push(id);
                events.push(payload);
            }
            drop(stmt);

            if !ids.is_empty() {
                let id_list = ids
                    .iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<String>>()
                    .join(",");
                tx.execute(
                    &format!("DELETE FROM event_buffer WHERE id IN ({})", id_list),
                    [],
                )?;
            }
            tx.commit()?;

            Ok::<Vec<String>, anyhow::Error>(events)
        })
        .await?
    }

    /// Count of events currently buffered (for heartbeat reporting).
    pub async fn len(&self) -> Result<usize> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().unwrap();
            let count: i64 =
                conn.query_row("SELECT COUNT(*) FROM event_buffer", [], |row| row.get(0))?;
            Ok::<usize, anyhow::Error>(count as usize)
        })
        .await?
    }

    /// Whether the buffer is empty.
    pub async fn is_empty(&self) -> Result<bool> {
        Ok(self.len().await? == 0)
    }
}
