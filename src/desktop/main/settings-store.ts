import { readFile, writeFile } from "node:fs/promises"
import { join } from "node:path"
import { app } from "electron"

import {
  defaultSettings,
  parseSettings,
  settingsSchema,
  type Settings,
} from "../shared/settings"

/**
 * Persistent settings store for the main process.
 *
 * Settings are stored as `settings.json` inside Electron's per-user
 * `userData` directory, which resolves to the OS-appropriate application
 * settings location:
 *   - macOS:   ~/Library/Application Support/Sonar/settings.json
 *   - Windows: %APPDATA%/Sonar/settings.json
 *   - Linux:   ~/.config/Sonar/settings.json
 */

function settingsFilePath(): string {
  return join(app.getPath("userData"), "settings.json")
}

// In-memory cache so the renderer can read settings without hitting disk each
// time. Loaded lazily on first access.
let cache: Settings | null = null

/**
 * Load settings from disk, validating against the schema and filling in
 * defaults for anything missing. Falls back to defaults if the file is absent
 * or unreadable/corrupt so the app always starts in a valid state.
 */
export async function loadSettings(): Promise<Settings> {
  if (cache) return cache

  try {
    const raw = await readFile(settingsFilePath(), "utf8")
    const parsed: unknown = JSON.parse(raw)
    cache = parseSettings(asRecord(parsed))
  } catch (error) {
    const err = error as NodeJS.ErrnoException
    if (err.code !== "ENOENT") {
      // Corrupt or invalid file — log and recover with defaults rather than
      // crashing. We do not overwrite the file here; it is rewritten on the
      // next successful save.
      console.error("Failed to load settings, using defaults:", error)
    }
    cache = { ...defaultSettings }
  }

  return cache
}

/**
 * Persist a full or partial settings object. The input is merged over the
 * current settings, re-validated, written to disk, and cached.
 * Returns the resulting validated settings.
 */
export async function saveSettings(input: unknown): Promise<Settings> {
  const current = await loadSettings()

  // Deep-parse the merged object so partial updates keep existing values and
  // still pass full schema validation.
  const merged = mergeSettings(current, input)
  const next = settingsSchema.parse(merged)

  await writeFile(settingsFilePath(), JSON.stringify(next, null, 2), "utf8")
  cache = next
  return next
}

/** Shallow-merge each known top-level section (general, auth, ...). */
function mergeSettings(current: Settings, input: unknown): unknown {
  if (typeof input !== "object" || input === null) return current

  const patch = input as Record<string, unknown>
  return {
    ...current,
    general: { ...current.general, ...(asRecord(patch.general)) },
    auth: { ...current.auth, ...(asRecord(patch.auth)) },
  }
}

function asRecord(value: unknown): Record<string, unknown> {
  return typeof value === "object" && value !== null
    ? (value as Record<string, unknown>)
    : {}
}
