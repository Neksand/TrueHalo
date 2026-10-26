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
    #[serde(default)]
    pub usage_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageTotals {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub responses: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRow {
    pub provider: String,
    pub model: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub responses: i64,
    pub last_used: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsage {
    pub day: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSummary {
    pub today: UsageTotals,
    pub all_time: UsageTotals,
    pub rows: Vec<UsageRow>,
    pub daily: Vec<DailyUsage>,
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
                usage_json TEXT,
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

        // Older databases predate usage tracking. Add the column in place so
        // existing history survives the upgrade.
        let has_usage_column = {
            let mut stmt = conn.prepare("PRAGMA table_info(messages)")?;
            let mut rows = stmt.query([])?;
            let mut found = false;
            while let Some(row) = rows.next()? {
                let name: String = row.get(1)?;
                if name == "usage_json" {
                    found = true;
                    break;
                }
            }
            found
        };
        if !has_usage_column {
            conn.execute("ALTER TABLE messages ADD COLUMN usage_json TEXT", [])?;
        }

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

    pub fn save_message(
        &self,
        id: &str,
        session_id: &str,
        role: &str,
        content_json: &str,
        usage_json: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono_now();

        conn.execute(
            "INSERT INTO messages (id, session_id, role, content_json, timestamp, usage_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, session_id, role, content_json, now, usage_json],
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
            "SELECT id, session_id, role, content_json, timestamp, usage_json FROM messages WHERE session_id = ?1 ORDER BY timestamp ASC",
        )?;

        let messages = stmt
            .query_map(params![session_id], |row| {
                Ok(MessageRecord {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    content_json: row.get(3)?,
                    timestamp: row.get(4)?,
                    usage_json: row.get(5)?,
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

    /// Aggregate token usage for the dashboard. Cost is resolved in the UI,
    /// because only the provider catalogs know their prices.
    pub fn usage_summary(&self) -> Result<UsageSummary> {
        let conn = self.conn.lock().unwrap();

        let totals_query = |where_clause: &str| -> Result<UsageTotals> {
            let sql = format!(
                "SELECT COALESCE(SUM(CAST(json_extract(usage_json, '$.prompt_tokens') AS INTEGER)), 0),
                        COALESCE(SUM(CAST(json_extract(usage_json, '$.completion_tokens') AS INTEGER)), 0),
                        COUNT(*)
                 FROM messages
                 WHERE usage_json IS NOT NULL {where_clause}"
            );
            let mut stmt = conn.prepare(&sql)?;
            let totals = stmt.query_row([], |row| {
                Ok(UsageTotals {
                    prompt_tokens: row.get(0)?,
                    completion_tokens: row.get(1)?,
                    responses: row.get(2)?,
                })
            })?;
            Ok(totals)
        };

        let today = totals_query(
            "AND date(timestamp / 1000, 'unixepoch', 'localtime') = date('now', 'localtime')",
        )?;
        let all_time = totals_query("")?;

        let mut rows_stmt = conn.prepare(
            "SELECT COALESCE(json_extract(m.usage_json, '$.provider'), s.provider),
                    COALESCE(json_extract(m.usage_json, '$.model'), s.model),
                    COALESCE(SUM(CAST(json_extract(m.usage_json, '$.prompt_tokens') AS INTEGER)), 0),
                    COALESCE(SUM(CAST(json_extract(m.usage_json, '$.completion_tokens') AS INTEGER)), 0),
                    COUNT(*),
                    MAX(m.timestamp)
             FROM messages m
             JOIN sessions s ON s.id = m.session_id
             WHERE m.usage_json IS NOT NULL
             GROUP BY 1, 2
             ORDER BY 6 DESC",
        )?;
        let rows = rows_stmt
            .query_map([], |row| {
                Ok(UsageRow {
                    provider: row.get(0)?,
                    model: row.get(1)?,
                    prompt_tokens: row.get(2)?,
                    completion_tokens: row.get(3)?,
                    responses: row.get(4)?,
                    last_used: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut daily_stmt = conn.prepare(
            "SELECT date(timestamp / 1000, 'unixepoch', 'localtime') AS day,
                    COALESCE(SUM(CAST(json_extract(usage_json, '$.prompt_tokens') AS INTEGER)), 0),
                    COALESCE(SUM(CAST(json_extract(usage_json, '$.completion_tokens') AS INTEGER)), 0)
             FROM messages
             WHERE usage_json IS NOT NULL
             GROUP BY day
             ORDER BY day DESC
             LIMIT 7",
        )?;
        let mut daily: Vec<DailyUsage> = daily_stmt
            .query_map([], |row| {
                Ok(DailyUsage {
                    day: row.get(0)?,
                    prompt_tokens: row.get(1)?,
                    completion_tokens: row.get(2)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        daily.reverse();

        Ok(UsageSummary {
            today,
            all_time,
            rows,
            daily,
        })
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
            None,
        )
        .unwrap();
        db.save_message(
            "msg-2",
            "sess-1",
            "assistant",
            r#"[{"type":"text","text":"hi!"}]"#,
            Some(r#"{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15,"provider":"openrouter","model":"claude-3.5-sonnet"}"#),
        )
        .unwrap();

        let msgs = db.get_session_messages("sess-1").unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[1].role, "assistant");
        assert!(msgs[1].usage_json.is_some());

        let usage = db.usage_summary().unwrap();
        assert_eq!(usage.all_time.prompt_tokens, 10);
        assert_eq!(usage.all_time.completion_tokens, 5);
        assert_eq!(usage.all_time.responses, 1);
        assert_eq!(usage.today.responses, 1);
        assert_eq!(usage.rows.len(), 1);
        assert_eq!(usage.rows[0].model, "claude-3.5-sonnet");

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
            None,
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
