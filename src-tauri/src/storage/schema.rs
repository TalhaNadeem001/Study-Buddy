use anyhow::Result;
use rusqlite::Connection;

/// Create the schema if absent: a `notes` table, an FTS5 mirror kept in sync by
/// triggers, a single-row `settings` blob, and a `cards` table reserved for the
/// Phase 4 spaced-repetition scheduler.
pub fn init(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS notes (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            path       TEXT NOT NULL UNIQUE,
            title      TEXT NOT NULL,
            body       TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts
            USING fts5(title, body, content='notes', content_rowid='id');

        CREATE TRIGGER IF NOT EXISTS notes_ai AFTER INSERT ON notes BEGIN
            INSERT INTO notes_fts(rowid, title, body)
            VALUES (new.id, new.title, new.body);
        END;

        CREATE TRIGGER IF NOT EXISTS notes_ad AFTER DELETE ON notes BEGIN
            INSERT INTO notes_fts(notes_fts, rowid, title, body)
            VALUES ('delete', old.id, old.title, old.body);
        END;

        CREATE TRIGGER IF NOT EXISTS notes_au AFTER UPDATE ON notes BEGIN
            INSERT INTO notes_fts(notes_fts, rowid, title, body)
            VALUES ('delete', old.id, old.title, old.body);
            INSERT INTO notes_fts(rowid, title, body)
            VALUES (new.id, new.title, new.body);
        END;

        CREATE TABLE IF NOT EXISTS settings (
            id   INTEGER PRIMARY KEY CHECK (id = 1),
            data TEXT NOT NULL
        );

        -- One row per voice/text capture: the raw transcript and the AI-written
        -- note, surfaced in the "Voice" tab.
        CREATE TABLE IF NOT EXISTS captures (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            transcript TEXT NOT NULL,
            content    TEXT NOT NULL,
            path       TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        -- Reserved for Phase 4 (fsrs-rs spaced repetition). Not used yet.
        CREATE TABLE IF NOT EXISTS cards (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            note_id    INTEGER REFERENCES notes(id) ON DELETE CASCADE,
            question   TEXT NOT NULL,
            answer     TEXT,
            due_at     INTEGER,
            created_at INTEGER NOT NULL
        );
        "#,
    )?;
    Ok(())
}
