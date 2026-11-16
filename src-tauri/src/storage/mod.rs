pub mod schema;

use crate::settings::Settings;
use anyhow::{anyhow, Result};
use rusqlite::{params, Connection, Row};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// A note row as stored in SQLite. Serialized camelCase for the IPC boundary.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteRecord {
    pub id: i64,
    pub path: String,
    pub title: String,
    pub body: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A capture: the raw transcript and the AI-written note. Shown in the Voice tab.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRecord {
    pub id: i64,
    pub transcript: String,
    pub content: String,
    pub path: String,
    pub created_at: i64,
}

/// Owns the single SQLite connection (behind a Mutex) and is held in Tauri state.
pub struct Storage {
    conn: Mutex<Connection>,
}

pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl Storage {
    pub fn new(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        schema::init(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn load_settings(&self) -> Result<Settings> {
        let conn = self.lock()?;
        let json: Option<String> = conn
            .query_row("SELECT data FROM settings WHERE id = 1", [], |r| r.get(0))
            .ok();
        Ok(match json {
            Some(s) => serde_json::from_str::<Settings>(&s).unwrap_or_default(),
            None => Settings::default(),
        })
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<Settings> {
        let normalized = settings.clone().normalized();
        let json = serde_json::to_string(&normalized)?;
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO settings (id, data) VALUES (1, ?1)
             ON CONFLICT(id) DO UPDATE SET data = excluded.data",
            params![json],
        )?;
        Ok(normalized)
    }

    /// Insert a note, or update the row with the same path. FTS stays in sync
    /// via triggers. Returns the row id.
    pub fn upsert_note(&self, path: &str, title: &str, body: &str) -> Result<i64> {
        let conn = self.lock()?;
        let now = now_millis();
        let existing: Option<i64> = conn
            .query_row("SELECT id FROM notes WHERE path = ?1", params![path], |r| {
                r.get(0)
            })
            .ok();
        match existing {
            Some(id) => {
                conn.execute(
                    "UPDATE notes SET title = ?1, body = ?2, updated_at = ?3 WHERE id = ?4",
                    params![title, body, now, id],
                )?;
                Ok(id)
            }
            None => {
                conn.execute(
                    "INSERT INTO notes (path, title, body, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?4)",
                    params![path, title, body, now],
                )?;
                Ok(conn.last_insert_rowid())
            }
        }
    }

    pub fn list_notes(&self, limit: i64) -> Result<Vec<NoteRecord>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, path, title, body, created_at, updated_at
             FROM notes ORDER BY updated_at DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], map_note)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Full-text search via FTS5. Each whitespace term becomes a prefix query,
    /// OR'd together. An empty query returns the most recent notes.
    pub fn search_notes(&self, query: &str) -> Result<Vec<NoteRecord>> {
        let terms: Vec<String> = query
            .split_whitespace()
            .map(|t| t.chars().filter(|c| c.is_alphanumeric()).collect::<String>())
            .filter(|t| !t.is_empty())
            .map(|t| format!("{t}*"))
            .collect();
        if terms.is_empty() {
            return self.list_notes(200);
        }
        let match_query = terms.join(" OR ");
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT n.id, n.path, n.title, n.body, n.created_at, n.updated_at
             FROM notes_fts f JOIN notes n ON n.id = f.rowid
             WHERE notes_fts MATCH ?1 ORDER BY rank LIMIT 200",
        )?;
        let rows = stmt
            .query_map(params![match_query], map_note)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn insert_capture(&self, transcript: &str, content: &str, path: &str) -> Result<CaptureRecord> {
        let conn = self.lock()?;
        let now = now_millis();
        conn.execute(
            "INSERT INTO captures (transcript, content, path, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![transcript, content, path, now],
        )?;
        Ok(CaptureRecord {
            id: conn.last_insert_rowid(),
            transcript: transcript.to_string(),
            content: content.to_string(),
            path: path.to_string(),
            created_at: now,
        })
    }

    pub fn list_captures(&self, limit: i64) -> Result<Vec<CaptureRecord>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT id, transcript, content, path, created_at
             FROM captures ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], |r| {
                Ok(CaptureRecord {
                    id: r.get(0)?,
                    transcript: r.get(1)?,
                    content: r.get(2)?,
                    path: r.get(3)?,
                    created_at: r.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| anyhow!("storage lock poisoned"))
    }
}

fn map_note(r: &Row) -> rusqlite::Result<NoteRecord> {
    Ok(NoteRecord {
        id: r.get(0)?,
        path: r.get(1)?,
        title: r.get(2)?,
        body: r.get(3)?,
        created_at: r.get(4)?,
        updated_at: r.get(5)?,
    })
}
