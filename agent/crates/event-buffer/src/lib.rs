use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

/// Local SQLite-backed buffer for JSON-encoded AgentEvent objects.
/// Used when the fleet server is unreachable.
pub struct EventBuffer {
    conn: Connection,
}

impl EventBuffer {
    /// Open or create the SQLite database at the given path.
    /// Creates the event_buffer table if it doesn't exist.
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS event_buffer (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                payload    JSON    NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(Self { conn })
    }

    /// Store a JSON-encoded AgentEvent.
    /// The string comes from serde_json::to_string().
    pub fn push(&self, event_json: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        self.conn.execute(
            "INSERT INTO event_buffer (payload, created_at) VALUES (?1, ?2)",
            rusqlite::params![event_json, now],
        )?;
        Ok(())
    }

    /// Read and remove the oldest `batch_size` events.
    /// Returns raw JSON strings that can be parsed back.
    pub fn drain(&self, batch_size: usize) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, payload FROM event_buffer ORDER BY id ASC LIMIT ?1")?;

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

        if !ids.is_empty() {
            let id_list = ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(",");
            self.conn.execute(
                &format!("DELETE FROM event_buffer WHERE id IN ({})", id_list),
                [],
            )?;
        }

        Ok(events)
    }

    /// Count of events currently buffered (for heartbeat reporting).
    pub fn len(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM event_buffer", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }
}
