import { globalShortcut } from "electron"

import type { ShortcutSettings } from "../shared/settings"
import { getCachedSettings } from "./settings-store"
import { cancelRecording, toggleRecording } from "./transcription"

let active: ShortcutSettings | null = null

function register(settings: ShortcutSettings): string[] {
  const registrations: Array<[string, () => void]> = [
    [settings.transcribe, () => void toggleRecording(true)],
    [settings.cancel, cancelRecording],
  ]
  const unavailable: string[] = []

  for (const [accelerator, handler] of registrations) {
    try {
      if (!globalShortcut.register(accelerator, handler)) {
        unavailable.push(accelerator)
      }
    } catch {
      unavailable.push(accelerator)
    }
  }
  return unavailable
}

export function registerShortcuts(
  settings: ShortcutSettings = getCachedSettings().shortcuts,
  allowPartial = false
): void {
  const previous = active
  globalShortcut.unregisterAll()
  const unavailable = register(settings)

  if (unavailable.length > 0 && !allowPartial) {
    globalShortcut.unregisterAll()
    if (previous) {
      const failedToRestore = register(previous)
      if (failedToRestore.length === 0) {
        active = previous
      } else {
        active = null
      }
    }
    throw new Error(`Shortcut is unavailable: ${unavailable.join(", ")}`)
  }

  active = settings
  if (unavailable.length > 0) {
    console.warn(`Global shortcuts unavailable: ${unavailable.join(", ")}`)
  }
}

export function unregisterShortcuts(): void {
  globalShortcut.unregisterAll()
  active = null
}
