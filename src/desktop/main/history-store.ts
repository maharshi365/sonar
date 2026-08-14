import { join } from "node:path"
import { DatabaseSync, type StatementResultingChanges } from "node:sqlite"
import { app } from "electron"

import type { HistoryEntry, HistoryPage } from "../shared/history"
import { migrateHistoryDatabase } from "./history-migrations"

const DEFAULT_PAGE_SIZE = 30
const MAX_PAGE_SIZE = 100

let database: DatabaseSync | null = null

function getDatabase(): DatabaseSync {
  if (database) return database

  const next = new DatabaseSync(join(app.getPath("userData"), "history.db"))
  try {
    migrateHistoryDatabase(next)
    next.exec("PRAGMA journal_mode = WAL;")
  } catch (error) {
    next.close()
    throw error
  }

  database = next
  return next
}

function mapEntry(row: Record<string, unknown>): HistoryEntry {
  return {
    id: Number(row.id),
    createdAt: Number(row.created_at),
    text: String(row.text),
    modelId: String(row.model_id),
  }
}

export function saveHistoryEntry(text: string, modelId: string): HistoryEntry {
  const createdAt = Date.now()
  const result = getDatabase()
    .prepare(
      "INSERT INTO transcription_history (created_at, text, model_id) VALUES (?, ?, ?)"
    )
    .run(createdAt, text, modelId)

  return {
    id: Number(result.lastInsertRowid),
    createdAt,
    text,
    modelId,
  }
}

export function listHistoryEntries(
  cursor?: number,
  requestedLimit = DEFAULT_PAGE_SIZE
): HistoryPage {
  const limit = Math.max(1, Math.min(requestedLimit, MAX_PAGE_SIZE))
  const fetchLimit = limit + 1
  const statement = cursor
    ? getDatabase().prepare(`
        SELECT id, created_at, text, model_id
        FROM transcription_history
        WHERE id < ?
        ORDER BY id DESC
        LIMIT ?
      `)
    : getDatabase().prepare(`
        SELECT id, created_at, text, model_id
        FROM transcription_history
        ORDER BY id DESC
        LIMIT ?
      `)

  const rows = (cursor
    ? statement.all(cursor, fetchLimit)
    : statement.all(fetchLimit)) as Record<string, unknown>[]
  const hasMore = rows.length > limit

  return {
    entries: rows.slice(0, limit).map(mapEntry),
    hasMore,
  }
}

export function deleteHistoryEntry(id: number): boolean {
  const result: StatementResultingChanges = getDatabase()
    .prepare("DELETE FROM transcription_history WHERE id = ?")
    .run(id)
  return result.changes > 0
}

export function clearHistory(): void {
  getDatabase().exec("DELETE FROM transcription_history")
}

export function closeHistoryStore(): void {
  database?.close()
  database = null
}
