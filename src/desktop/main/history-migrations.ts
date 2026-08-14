import type { DatabaseSync } from "node:sqlite"

// Append migrations only. The array index plus one is the schema version stored
// in SQLite's user_version pragma.
const migrations = [
  `
    CREATE TABLE transcription_history (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      created_at INTEGER NOT NULL,
      text TEXT NOT NULL,
      model_id TEXT NOT NULL
    );
  `,
] as const

export function migrateHistoryDatabase(database: DatabaseSync): void {
  const versionRow = database.prepare("PRAGMA user_version").get() as {
    user_version: number
  }
  const currentVersion = versionRow.user_version

  if (!Number.isSafeInteger(currentVersion) || currentVersion < 0) {
    throw new Error(`History database has invalid schema version ${currentVersion}.`)
  }
  if (currentVersion > migrations.length) {
    throw new Error(
      `History database version ${currentVersion} is newer than supported version ${migrations.length}.`
    )
  }

  for (let index = currentVersion; index < migrations.length; index += 1) {
    const targetVersion = index + 1
    database.exec("BEGIN IMMEDIATE;")
    try {
      database.exec(migrations[index])
      database.exec(`PRAGMA user_version = ${targetVersion};`)
      database.exec("COMMIT;")
    } catch (error) {
      try {
        database.exec("ROLLBACK;")
      } catch {
        // Preserve the migration error; the connection is closed by the caller.
      }
      throw new Error(`Failed to migrate history database to version ${targetVersion}.`, {
        cause: error,
      })
    }
  }
}
