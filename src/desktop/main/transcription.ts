import { BrowserWindow } from "electron"

// The native Rust addon. Only the main process may load it.
import * as core from "@sonar/core"

import { IpcChannels } from "../shared/ipc"
import type { StreamText } from "../shared/transcription"
import { ensureModelsInitialized, listModels } from "./models"
import { hideOverlay, sendToOverlay, showOverlay } from "./overlay"
import { getCachedSettings, loadSettings } from "./settings-store"

/**
 * Live transcription controller for the main process.
 *
 * Bridges the Rust pipeline (`@sonar/core`) to the UI: resolves the selected
 * model, starts/stops recording, and forwards live text + audio levels to the
 * dock overlay and the main window. Recording state is tracked here so the
 * global shortcut and the on-screen button share one source of truth.
 */

let recording = false

/** Broadcast to every renderer window (main + overlay). */
function broadcast(channel: string, ...args: unknown[]): void {
  for (const window of BrowserWindow.getAllWindows()) {
    if (!window.isDestroyed()) window.webContents.send(channel, ...args)
  }
}

export function isRecording(): boolean {
  return recording
}

/** Resolve the selected model's id + filename, or throw a user-facing error. */
function resolveSelectedModel(): { id: string; filename: string } {
  ensureModelsInitialized()
  const settings = getCachedSettings()
  const selectedId = settings.general.ttsModel.trim()

  const models = listModels()
  const downloaded = models.filter((m) => m.isDownloaded)

  if (downloaded.length === 0) {
    throw new Error("No speech model installed. Download one from the Models page.")
  }

  // Prefer the explicitly selected model; otherwise fall back to the first
  // downloaded one so recording still works before a choice is made.
  const chosen = downloaded.find((m) => m.id === selectedId) ?? downloaded[0]
  return { id: chosen.id, filename: chosen.filename }
}

/** Warm the settings cache so `resolveSelectedModel` sees the latest choice. */
export async function refreshSettingsCache(): Promise<void> {
  await loadSettings()
}

/** Start a recording + live transcription session. */
export function startRecording(): void {
  if (recording) return

  let model: { id: string; filename: string }
  try {
    model = resolveSelectedModel()
  } catch (error) {
    broadcast(IpcChannels.transcriptionError, (error as Error).message)
    return
  }

  try {
    core.startTranscription(
      model.id,
      model.filename,
      (text: StreamText) => {
        sendToOverlay(IpcChannels.transcriptionText, text)
        broadcast(IpcChannels.transcriptionText, text)
      },
      (levels: number[]) => {
        sendToOverlay(IpcChannels.transcriptionLevels, levels)
      },
    )
  } catch (error) {
    broadcast(IpcChannels.transcriptionError, (error as Error).message)
    return
  }

  recording = true
  showOverlay()
  broadcast(IpcChannels.transcriptionState, true)
}

/** Stop recording, transcribe, and broadcast the final text. */
export async function stopRecording(): Promise<string> {
  if (!recording) return ""
  recording = false
  broadcast(IpcChannels.transcriptionState, false)

  try {
    const text = await core.stopTranscription()
    broadcast(IpcChannels.transcriptionResult, text)
    return text
  } catch (error) {
    broadcast(IpcChannels.transcriptionError, (error as Error).message)
    return ""
  } finally {
    hideOverlay()
  }
}

/** Cancel an in-flight recording, discarding the transcript. */
export function cancelRecording(): void {
  if (!recording) return
  recording = false
  try {
    core.cancelTranscription()
  } catch {
    // best-effort
  }
  broadcast(IpcChannels.transcriptionState, false)
  hideOverlay()
}

/** Toggle recording; returns the resulting state. */
export async function toggleRecording(): Promise<boolean> {
  if (recording) {
    await stopRecording()
    return false
  }
  startRecording()
  return recording
}
