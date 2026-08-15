import { BrowserWindow, clipboard, systemPreferences } from "electron"

// The native Rust addon. Only the main process may load it.
import * as core from "@sonar/core"

import { IpcChannels } from "../shared/ipc"
import type { StreamText, TranscriptionState } from "../shared/transcription"
import { saveHistoryEntry } from "./history-store"
import { ensureModelsInitialized, listModels } from "./models"
import { hideOverlay, sendToOverlay, showOverlay } from "./overlay"
import { getCachedSettings, loadSettings } from "./settings-store"

/**
 * Live transcription controller for the main process.
 *
 * Bridges the Rust pipeline (`@sonar/core`) to the UI: resolves the selected
 * model, starts/stops recording, and forwards live text + audio levels to the
 * dock overlay and the main window. Lifecycle state is tracked here so the
 * global shortcut and the on-screen button share one source of truth.
 */

let state: TranscriptionState = "idle"
let activeModelId: string | null = null
let insertOnComplete = false
let unloadTimer: NodeJS.Timeout | null = null
let cancelBufferWait: (() => void) | null = null
let stopCancelled = false

/** Broadcast to every renderer window (main + overlay). */
function broadcast(channel: string, ...args: unknown[]): void {
  for (const window of BrowserWindow.getAllWindows()) {
    if (!window.isDestroyed()) window.webContents.send(channel, ...args)
  }
}

export function isRecording(): boolean {
  return state === "recording"
}

function setState(next: TranscriptionState): void {
  state = next
  broadcast(IpcChannels.transcriptionState, next)
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
export function startRecording(shouldInsert = false): void {
  if (state !== "idle") return
  if (unloadTimer) {
    clearTimeout(unloadTimer)
    unloadTimer = null
  }

  let model: { id: string; filename: string }
  try {
    model = resolveSelectedModel()
  } catch (error) {
    broadcast(IpcChannels.transcriptionError, (error as Error).message)
    return
  }
  const settings = getCachedSettings()

  try {
    core.startTranscription(
      model.id,
      model.filename,
      {
        inputDeviceId: settings.audio.inputDeviceId || undefined,
        customWords: settings.transcription.customWords,
        fillerWordRemoval: settings.transcription.fillerWordRemoval,
        customFillerWords: settings.transcription.customFillerWords,
        wordCorrectionThreshold:
          settings.transcription.wordCorrectionThreshold,
        accelerator: settings.inference.accelerator,
        gpuDeviceId: settings.inference.gpuDeviceId,
      },
      (text: StreamText) => {
        sendToOverlay(IpcChannels.transcriptionText, text)
        broadcast(IpcChannels.transcriptionText, text)
      },
      (levels: number[]) => {
        sendToOverlay(IpcChannels.transcriptionLevels, levels)
      },
    )
    activeModelId = model.id
    insertOnComplete = shouldInsert
  } catch (error) {
    broadcast(IpcChannels.transcriptionError, (error as Error).message)
    return
  }

  showOverlay()
  setState("recording")
}

/** Stop recording, transcribe, and broadcast the final text. */
export async function stopRecording(): Promise<string> {
  if (state !== "recording") return ""
  setState("transcribing")
  const shouldInsert = insertOnComplete
  const settings = getCachedSettings()
  stopCancelled = false

  try {
    if (settings.transcription.extraRecordingBufferMs > 0) {
      await waitForRecordingBuffer(settings.transcription.extraRecordingBufferMs)
      if (stopCancelled) return ""
    }
    const text = await core.stopTranscription()
    if (stopCancelled) return ""
    if (text.trim()) {
      try {
        const saved = saveHistoryEntry(
          text,
          activeModelId ?? "unknown",
          settings.general.historyLimit
        )
        if (saved) broadcast(IpcChannels.historyChanged)
      } catch (error) {
        console.error("Failed to save transcription history:", error)
      }
      if (shouldInsert || settings.output.method === "clipboard") {
        try {
          if (
            settings.output.method === "paste" &&
            process.platform === "darwin" &&
            !systemPreferences.isTrustedAccessibilityClient(true)
          ) {
            throw new Error(
              "Accessibility permission is required. Enable Sonar in System Settings > Privacy & Security > Accessibility."
            )
          }
          const outputText = settings.output.appendTrailingSpace
            ? `${text} `
            : text
          if (settings.output.method === "paste") {
            core.insertText(outputText)
            if (settings.output.autoSubmit) {
              await sleep(100)
              core.submitKey(settings.output.autoSubmitKey)
            }
          } else if (settings.output.method === "clipboard") {
            clipboard.writeText(outputText)
          }
        } catch (error) {
          const message = `Transcription completed, but text insertion failed: ${(error as Error).message}`
          console.error(message)
          broadcast(IpcChannels.transcriptionError, message)
        }
      }
    }
    broadcast(IpcChannels.transcriptionResult, text)
    return text
  } catch (error) {
    broadcast(IpcChannels.transcriptionError, (error as Error).message)
    return ""
  } finally {
    activeModelId = null
    insertOnComplete = false
    setState("idle")
    hideOverlay()
    scheduleModelUnload(getCachedSettings().general.modelUnloadTimeout)
  }
}

/** Cancel an in-flight recording, discarding the transcript. */
export function cancelRecording(): void {
  if (state !== "recording" && state !== "transcribing") return
  if (state === "transcribing") {
    stopCancelled = true
    cancelBufferWait?.()
    try {
      core.cancelTranscription()
    } catch {
      // best-effort
    }
    return
  }
  try {
    core.cancelTranscription()
  } catch {
    // best-effort
  }
  setState("idle")
  activeModelId = null
  insertOnComplete = false
  hideOverlay()
  scheduleModelUnload(getCachedSettings().general.modelUnloadTimeout)
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function waitForRecordingBuffer(ms: number): Promise<void> {
  return new Promise((resolve) => {
    const timer = setTimeout(() => {
      cancelBufferWait = null
      resolve()
    }, ms)
    cancelBufferWait = () => {
      clearTimeout(timer)
      cancelBufferWait = null
      resolve()
    }
  })
}

export function scheduleModelUnload(timeout: string): void {
  if (unloadTimer) {
    clearTimeout(unloadTimer)
    unloadTimer = null
  }
  if (state !== "idle") return
  if (timeout === "never") return
  const delays: Record<string, number> = {
    immediately: 0,
    "2m": 2 * 60_000,
    "5m": 5 * 60_000,
    "10m": 10 * 60_000,
    "15m": 15 * 60_000,
    "1h": 60 * 60_000,
  }
  const delay = delays[timeout]
  if (delay === undefined) return
  unloadTimer = setTimeout(() => {
    unloadTimer = null
    try {
      core.unloadModel()
    } catch (error) {
      console.error("Failed to unload transcription model", error)
    }
  }, delay)
}

/** Toggle recording; returns the resulting state. */
export async function toggleRecording(shouldInsert = false): Promise<boolean> {
  if (state === "recording") {
    await stopRecording()
    return false
  }
  if (state === "transcribing") return false
  startRecording(shouldInsert)
  return isRecording()
}
