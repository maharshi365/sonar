import { globalShortcut } from "electron"

import { toggleRecording } from "./transcription"

/**
 * Global keyboard shortcut for hands-free dictation.
 *
 * The accelerator matches the hint shown on the home page. Electron's
 * `globalShortcut` fires while any app is focused, so pressing it toggles a
 * recording session and the dock overlay appears/disappears accordingly.
 *
 * NOTE: this is a toggle (press to start, press again to stop), not push-to-
 * talk — Electron's globalShortcut doesn't expose key-up events. A push-to-talk
 * backend (rdev-style) can replace this later without touching the UI.
 */
const ACCELERATOR = "CommandOrControl+Shift+Space"

export function registerShortcuts(): void {
  const ok = globalShortcut.register(ACCELERATOR, () => {
    void toggleRecording()
  })
  if (!ok) {
    console.error(`Failed to register global shortcut: ${ACCELERATOR}`)
  }
}

export function unregisterShortcuts(): void {
  globalShortcut.unregisterAll()
}
