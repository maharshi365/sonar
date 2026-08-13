import { join } from "node:path"
import { app, BrowserWindow } from "electron"

// The native Rust addon. Only the main process may load it.
import * as core from "@sonar/core"

import { IpcChannels } from "../shared/ipc"
import type { ModelDownloadProgress, ModelStatus } from "../shared/models"
import { loadSettings, saveSettings } from "./settings-store"

/**
 * Model manager for the main process.
 *
 * Thin wrapper over the Rust core (`@sonar/core`). Resolves the models
 * directory from Electron's userData path, forwards download progress to all
 * renderer windows, and reads the optional Hugging Face token from settings.
 *
 * Models are stored under:
 *   <userData>/models/
 * e.g. macOS: ~/Library/Application Support/Sonar/models/
 */

let initialized = false

/** Directory where model files live. */
function modelsDir(): string {
  return join(app.getPath("userData"), "models")
}

/** Initialize the Rust core once. Idempotent. */
function ensureInitialized(): void {
  if (initialized) return
  core.initModels(modelsDir())
  initialized = true
}

/** Broadcast a download-progress event to every open window. */
function broadcastProgress(progress: ModelDownloadProgress): void {
  for (const window of BrowserWindow.getAllWindows()) {
    if (!window.isDestroyed()) {
      window.webContents.send(IpcChannels.modelsProgress, progress)
    }
  }
}

/** List every catalog model with its on-disk status. */
export function listModels(): ModelStatus[] {
  ensureInitialized()
  return core.listModels()
}

/**
 * Download a model by id. Progress is streamed to renderers via the
 * `models:progress` channel. Uses the Hugging Face token from settings when
 * present. Resolves once the download completes.
 */
export async function downloadModel(modelId: string): Promise<void> {
  ensureInitialized()

  const settings = await loadSettings()
  const token = settings.auth.huggingFaceToken.trim()

  await core.downloadModel(modelId, token.length > 0 ? token : null, (progress) => {
    broadcastProgress(progress)
  })
}

/** Cancel an in-flight download. Returns true if one was actually running. */
export function cancelDownload(modelId: string): boolean {
  ensureInitialized()
  return core.cancelDownload(modelId)
}

/**
 * Remove a downloaded model (and any partial) from disk.
 *
 * If the removed model was selected as the default speech model, the setting is
 * cleared so it no longer points at a missing model.
 */
export async function removeModel(modelId: string): Promise<void> {
  ensureInitialized()
  await core.removeModel(modelId)

  const settings = await loadSettings()
  if (settings.general.ttsModel === modelId) {
    await saveSettings({ general: { ttsModel: "" } })
  }
}
