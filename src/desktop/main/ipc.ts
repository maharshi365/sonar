import { BrowserWindow, ipcMain } from "electron"

import { IpcChannels } from "../shared/ipc"
import {
  clearHistory,
  deleteHistoryEntry,
  listHistoryEntries,
} from "./history-store"
import {
  cancelDownload,
  downloadModel,
  listModels,
  removeModel,
} from "./models"
import { loadSettings, saveSettings } from "./settings-store"
import {
  cancelRecording,
  refreshSettingsCache,
  startRecording,
  stopRecording,
  toggleRecording,
} from "./transcription"

/**
 * Register all main-process IPC handlers. Called once during app startup.
 */
export function registerIpcHandlers(): void {
  ipcMain.handle(IpcChannels.settingsGet, () => loadSettings())
  ipcMain.handle(IpcChannels.settingsSet, async (_event, patch: unknown) => {
    const settings = await saveSettings(patch)
    // Keep the transcription controller's cached settings in sync so the next
    // recording uses the freshly selected model.
    await refreshSettingsCache()
    return settings
  })

  // Models. Download progress is pushed separately via webContents.send on the
  // `models:progress` channel (see main/models.ts).
  ipcMain.handle(IpcChannels.modelsList, () => listModels())
  ipcMain.handle(IpcChannels.modelsDownload, (_event, modelId: string) =>
    downloadModel(modelId)
  )
  ipcMain.handle(IpcChannels.modelsCancel, (_event, modelId: string) =>
    cancelDownload(modelId)
  )
  ipcMain.handle(IpcChannels.modelsRemove, (_event, modelId: string) =>
    removeModel(modelId)
  )

  // Transcription history.
  ipcMain.handle(
    IpcChannels.historyList,
    (_event, cursor?: unknown, limit?: unknown) =>
      listHistoryEntries(
        optionalPositiveInteger(cursor, "cursor"),
        optionalPositiveInteger(limit, "limit")
      )
  )
  ipcMain.handle(IpcChannels.historyDelete, (_event, id: number) => {
    if (!Number.isSafeInteger(id) || id <= 0) {
      throw new Error("Invalid history entry id")
    }
    const deleted = deleteHistoryEntry(id)
    if (deleted) broadcastHistoryChanged()
    return deleted
  })
  ipcMain.handle(IpcChannels.historyClear, () => {
    clearHistory()
    broadcastHistoryChanged()
  })

  // Live transcription.
  ipcMain.handle(IpcChannels.transcriptionToggle, () => toggleRecording())
  ipcMain.handle(IpcChannels.transcriptionStart, () => startRecording())
  ipcMain.handle(IpcChannels.transcriptionStop, () => stopRecording())
  ipcMain.handle(IpcChannels.transcriptionCancel, () => cancelRecording())
}

function broadcastHistoryChanged(): void {
  for (const window of BrowserWindow.getAllWindows()) {
    if (!window.isDestroyed()) {
      window.webContents.send(IpcChannels.historyChanged)
    }
  }
}

function optionalPositiveInteger(value: unknown, name: string): number | undefined {
  if (value === undefined) return undefined
  if (!Number.isSafeInteger(value) || (value as number) <= 0) {
    throw new Error(`Invalid history ${name}`)
  }
  return value as number
}
