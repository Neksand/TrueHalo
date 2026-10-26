use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub title: String,
    pub provider: String,
    pub model: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content_json: String,
    pub timestamp: i64,
}

pub struct DatabaseManager {
    conn: Mutex<Connection>,
}

impl DatabaseManager {
    pub fn new(db_path: PathBuf) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create app data directory")?;
        }

        let conn = Connection::open(&db_path)
            .with_context(|| format!("Failed to open SQLite database at {:?}", db_path))?;

        let manager = Self {
            conn: Mutex::new(conn),
        };
        manager.init_tables()?;
        Ok(manager)
    }

    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let manager = Self {
            conn: Mutex::new(conn),
        };
        manager.init_tables()?;
        Ok(manager)
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content_json TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS preferences (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, timestamp);
            ",
        )
        .context("Failed to initialize SQLite database tables")?;

        Ok(())
    }

    pub fn create_session(
        &self,
        id: &str,
        title: &str,
        provider: &str,
        model: &str,
    ) -> Result<SessionRecord> {
        let conn = self.conn.lock().unwrap();
        let now = chrono_now();

        conn.execute(
            "INSERT INTO sessions (id, title, provider, model, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, title, provider, model, now, now],
        )?;

        Ok(SessionRecord {
            id: id.to_string(),
            title: title.to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, provider, model, created_at, updated_at FROM sessions ORDER BY updated_at DESC",
        )?;

        let sessions = stmt
            .query_map([], |row| {
                Ok(SessionRecord {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    provider: row.get(2)?,
                    model: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(sessions)
    }

    pub fn rename_session(&self, session_id: &str, title: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET title = ?1, updated_at = ?2 WHERE id = ?3",
            params![title, chrono_now(), session_id],
        )?;
        Ok(())
    }

    /// Search session titles and message text.
    pub fn search_sessions(&self, query: &str) -> Result<Vec<SessionRecord>> {
        let conn = self.conn.lock().unwrap();
        let pattern = format!("%{}%", query.trim());
        let mut stmt = conn.prepare(
            "SELECT DISTINCT s.id, s.title, s.provider, s.model, s.created_at, s.updated_at
             FROM sessions s
             LEFT JOIN messages m ON m.session_id = s.id
             WHERE s.title LIKE ?1 OR m.content_json LIKE ?1
             ORDER BY s.updated_at DESC",
        )?;

        let sessions = stmt
            .query_map(params![pattern], |row| {
                Ok(SessionRecord {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    provider: row.get(2)?,
                    model: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(sessions)
    }

    pub fn save_message(        &self,
        id: &str,
        session_id: &str,
        role: &str,
        content_json: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono_now();

        conn.execute(
            "INSERT INTO messages (id, session_id, role, content_json, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, session_id, role, content_json, now],
        )?;

        conn.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
            params![now, session_id],
        )?;

        Ok(())
    }

    pub fn get_session_messages(&self, session_id: &str) -> Result<Vec<MessageRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, content_json, timestamp FROM messages WHERE session_id = ?1 ORDER BY timestamp ASC",
        )?;

        let messages = stmt
            .query_map(params![session_id], |row| {
                Ok(MessageRecord {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    content_json: row.get(3)?,
                    timestamp: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(messages)
    }

    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM messages WHERE session_id = ?1",
            params![session_id],
        )?;
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;
        Ok(())
    }

    pub fn clear_all_history(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM messages", [])?;
        conn.execute("DELETE FROM sessions", [])?;
        Ok(())
    }

    pub fn set_preference(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO preferences (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_preference(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM preferences WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            let val: String = row.get(0)?;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }
}

fn chrono_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_session_and_messages() {
        let db = DatabaseManager::new_in_memory().unwrap();

        let session = db
            .create_session("sess-1", "Test Chat", "openrouter", "claude-3.5-sonnet")
            .unwrap();
        assert_eq!(session.id, "sess-1");

        let sessions = db.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);

        db.save_message(
            "msg-1",
            "sess-1",
            "user",
            r#"[{"type":"text","text":"hello"}]"#,
        )
        .unwrap();
        db.save_message(
            "msg-2",
            "sess-1",
            "assistant",
            r#"[{"type":"text","text":"hi!"}]"#,
        )
        .unwrap();

        let msgs = db.get_session_messages("sess-1").unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[1].role, "assistant");

        db.delete_session("sess-1").unwrap();
        let remaining = db.list_sessions().unwrap();
        assert_eq!(remaining.len(), 0);
    }

    #[test]
    fn test_rename_and_search_sessions() {
        let db = DatabaseManager::new_in_memory().unwrap();
        db.create_session("sess-1", "Rust question", "openrouter", "model-a")
            .unwrap();
        db.create_session("sess-2", "Unrelated", "openai", "model-b")
            .unwrap();
        db.save_message(
            "msg-1",
            "sess-1",
            "user",
            r#"[{"type":"text","text":"explain the borrow checker"}]"#,
        )
        .unwrap();

        let by_title = db.search_sessions("Rust").unwrap();
        assert_eq!(by_title.len(), 1);
        assert_eq!(by_title[0].id, "sess-1");

        let by_message = db.search_sessions("borrow").unwrap();
        assert_eq!(by_message.len(), 1);
        assert_eq!(by_message[0].id, "sess-1");

        db.rename_session("sess-2", "Renamed chat").unwrap();
        let sessions = db.list_sessions().unwrap();
        let renamed = sessions.iter().find(|s| s.id == "sess-2").unwrap();
        assert_eq!(renamed.title, "Renamed chat");
    }
}
