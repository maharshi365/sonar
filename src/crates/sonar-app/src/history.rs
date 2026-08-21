use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: i64 = 1;
const DEFAULT_PAGE_SIZE: usize = 30;
const MAX_PAGE_SIZE: usize = 100;

const CREATE_HISTORY_TABLE: &str = r"
    CREATE TABLE transcription_history (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      created_at INTEGER NOT NULL,
      text TEXT NOT NULL,
      model_id TEXT NOT NULL
    );
";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: i64,
    pub created_at: i64,
    pub text: String,
    pub model_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub entries: Vec<HistoryEntry>,
    pub has_more: bool,
}

pub struct HistoryStore {
    connection: Connection,
}

impl HistoryStore {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let mut connection = Connection::open(path).context("failed to open history database")?;
        migrate(&mut connection)?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .context("failed to enable WAL for history database")?;
        Ok(Self { connection })
    }

    pub fn save(
        &self,
        text: &str,
        model_id: &str,
        history_limit: usize,
    ) -> Result<Option<HistoryEntry>> {
        if history_limit == 0 {
            return Ok(None);
        }

        let created_at = unix_time_millis()?;
        self.connection
            .execute(
                "INSERT INTO transcription_history (created_at, text, model_id) VALUES (?1, ?2, ?3)",
                params![created_at, text, model_id],
            )
            .context("failed to save history entry")?;
        let id = self.connection.last_insert_rowid();

        self.retain_newest(history_limit)?;

        Ok(Some(HistoryEntry {
            id,
            created_at,
            text: text.to_owned(),
            model_id: model_id.to_owned(),
        }))
    }

    pub fn list(&self, cursor: Option<i64>, requested_limit: Option<usize>) -> Result<HistoryPage> {
        let limit = requested_limit
            .unwrap_or(DEFAULT_PAGE_SIZE)
            .clamp(1, MAX_PAGE_SIZE);
        let fetch_limit =
            i64::try_from(limit.saturating_add(1)).context("history page size is too large")?;
        let sql = if cursor.is_some() {
            "SELECT id, created_at, text, model_id
             FROM transcription_history
             WHERE id < ?1
             ORDER BY id DESC
             LIMIT ?2"
        } else {
            "SELECT id, created_at, text, model_id
             FROM transcription_history
             ORDER BY id DESC
             LIMIT ?1"
        };
        let mut statement = self
            .connection
            .prepare(sql)
            .context("failed to prepare history query")?;
        let map_row = |row: &rusqlite::Row<'_>| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                created_at: row.get(1)?,
                text: row.get(2)?,
                model_id: row.get(3)?,
            })
        };
        let rows = match cursor {
            Some(cursor) => statement.query_map(params![cursor, fetch_limit], map_row)?,
            None => statement.query_map(params![fetch_limit], map_row)?,
        };
        let mut entries = rows
            .collect::<rusqlite::Result<Vec<_>>>()
            .context("failed to read history entries")?;
        let has_more = entries.len() > limit;
        entries.truncate(limit);

        Ok(HistoryPage { entries, has_more })
    }

    pub fn delete(&self, id: i64) -> Result<bool> {
        let changed = self
            .connection
            .execute("DELETE FROM transcription_history WHERE id = ?1", [id])
            .context("failed to delete history entry")?;
        Ok(changed > 0)
    }

    pub fn clear(&self) -> Result<()> {
        self.connection
            .execute("DELETE FROM transcription_history", [])
            .context("failed to clear history")?;
        Ok(())
    }

    pub fn prune(&self, limit: usize) -> Result<()> {
        if limit == 0 {
            return self.clear();
        }
        self.retain_newest(limit)
    }

    fn retain_newest(&self, limit: usize) -> Result<()> {
        let limit = i64::try_from(limit).context("history limit is too large")?;
        self.connection
            .execute(
                "DELETE FROM transcription_history
                 WHERE id NOT IN (
                   SELECT id FROM transcription_history ORDER BY id DESC LIMIT ?1
                 )",
                [limit],
            )
            .context("failed to prune history")?;
        Ok(())
    }
}

fn migrate(connection: &mut Connection) -> Result<()> {
    let current_version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .context("failed to read history database schema version")?;
    if current_version < 0 {
        bail!("history database has invalid schema version {current_version}");
    }
    if current_version > SCHEMA_VERSION {
        bail!(
            "history database version {current_version} is newer than supported version {SCHEMA_VERSION}"
        );
    }

    if current_version == 0 {
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .context("failed to begin history database migration")?;
        transaction
            .execute_batch(CREATE_HISTORY_TABLE)
            .context("failed to migrate history database to version 1")?;
        transaction
            .pragma_update(None, "user_version", SCHEMA_VERSION)
            .context("failed to set history database schema version")?;
        transaction
            .commit()
            .context("failed to commit history database migration")?;
    }
    Ok(())
}

fn unix_time_millis() -> Result<i64> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?;
    i64::try_from(elapsed.as_millis()).context("current time does not fit in SQLite integer")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Result<Self> {
            let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("sonar-history-{}-{sequence}", std::process::id()));
            fs::create_dir(&path)?;
            Ok(Self(path))
        }

        fn database_path(&self) -> PathBuf {
            self.0.join("history.db")
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn migrates_and_uses_compatible_schema() -> Result<()> {
        let directory = TestDirectory::new()?;
        let path = directory.database_path();
        let store = HistoryStore::new(&path)?;
        drop(store);

        let connection = Connection::open(path)?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        let journal_mode: String =
            connection.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
        assert_eq!(version, 1);
        assert_eq!(journal_mode, "wal");
        Ok(())
    }

    #[test]
    fn saves_pages_and_prunes_newest_entries() -> Result<()> {
        let directory = TestDirectory::new()?;
        let store = HistoryStore::new(directory.database_path())?;
        for index in 0..5 {
            store.save(&format!("entry {index}"), "model", 10)?;
        }

        let first = store.list(None, Some(2))?;
        assert_eq!(first.entries.len(), 2);
        assert!(first.has_more);
        assert_eq!(
            first.entries.first().map(|entry| entry.text.as_str()),
            Some("entry 4")
        );
        let second = store.list(first.entries.last().map(|entry| entry.id), Some(2))?;
        assert_eq!(
            second.entries.first().map(|entry| entry.text.as_str()),
            Some("entry 2")
        );

        store.prune(2)?;
        let remaining = store.list(None, None)?;
        assert_eq!(remaining.entries.len(), 2);
        assert!(!remaining.has_more);
        let newest_id = remaining.entries.first().map(|entry| entry.id);
        assert!(newest_id.is_some());
        assert!(store.delete(newest_id.unwrap_or_default())?);
        assert!(!store.delete(newest_id.unwrap_or_default())?);
        store.clear()?;
        assert!(store.list(None, None)?.entries.is_empty());
        Ok(())
    }

    #[test]
    fn zero_limit_does_not_save() -> Result<()> {
        let directory = TestDirectory::new()?;
        let store = HistoryStore::new(directory.database_path())?;
        assert!(store.save("ignored", "model", 0)?.is_none());
        assert!(store.list(None, None)?.entries.is_empty());
        Ok(())
    }
}
