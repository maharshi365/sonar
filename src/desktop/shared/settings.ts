import { z } from "zod"

/**
 * Shared settings schema for Sonar.
 *
 * This module is imported by the main process (to load/save/validate the
 * settings.json file) and by the renderer (for typing the settings object it
 * receives over IPC). It must not import anything Electron- or Node-specific
 * so it stays safe to bundle into the renderer.
 *
 * Conventions:
 * - Every setting MUST provide a default via `.default(...)`. This guarantees
 *   `settingsSchema.parse({})` always yields a complete, valid object and lets
 *   us tolerate missing/partial settings.json files.
 * - Group related settings into logical sub-objects (general, models, ...).
 */

/** General application preferences. */
const generalSettingsSchema = z.object({
  /**
   * Identifier of the default text-to-speech model to use.
   *
   * DEFAULT: empty string — no model is selected until the user installs one.
   * Once a model manager exists this should be validated against the set of
   * installed model IDs rather than being a free-form string.
   */
  ttsModel: z.string().default(""),
})

/** Root settings object persisted to settings.json. */
export const settingsSchema = z.object({
  // `.prefault({})` lets the whole section be omitted from settings.json while
  // still applying each field's individual default.
  general: generalSettingsSchema.prefault({}),
})

export type Settings = z.infer<typeof settingsSchema>
export type GeneralSettings = Settings["general"]

/**
 * The complete default settings object.
 *
 * Derived from the schema so defaults live in exactly one place (the schema).
 */
export const defaultSettings: Settings = settingsSchema.parse({})

/**
 * Parse a (possibly partial) settings object into a valid Settings object,
 * filling in defaults for anything missing. Throws if a present value has the
 * wrong type.
 */
export function parseSettings(input: Record<string, unknown> = {}): Settings {
  return settingsSchema.parse(input)
}
