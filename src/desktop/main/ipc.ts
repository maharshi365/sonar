import { BrowserWindow, ipcMain } from "electron"
import * as core from "@sonar/core"

import { IpcChannels } from "../shared/ipc"
import {
  clearHistory,
  deleteHistoryEntry,
  listHistoryEntries,
  pruneHistory,
} from "./history-store"
import {
  cancelDownload,
  downloadModel,
  listModels,
  removeModel,
} from "./models"
import { loadSettings, saveSettings } from "./settings-store"
import { registerShortcuts } from "./shortcuts"
import {
  cancelRecording,
  refreshSettingsCache,
  scheduleModelUnload,
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
    const previous = await loadSettings()
    const settings = await saveSettings(patch)
    if (
      settings.shortcuts.transcribe !== previous.shortcuts.transcribe ||
      settings.shortcuts.cancel !== previous.shortcuts.cancel
    ) {
      try {
        registerShortcuts(settings.shortcuts)
      } catch (error) {
        await saveSettings({ shortcuts: previous.shortcuts })
        throw error
      }
    }
    await refreshSettingsCache()
    if (settings.general.historyLimit !== previous.general.historyLimit) {
      pruneHistory(settings.general.historyLimit)
      broadcastHistoryChanged()
    }
    if (
      settings.general.modelUnloadTimeout !==
      previous.general.modelUnloadTimeout
    ) {
      scheduleModelUnload(settings.general.modelUnloadTimeout)
    }
    return settings
  })

  ipcMain.handle(IpcChannels.audioInputDevices, () => core.listInputDevices())
  ipcMain.handle(IpcChannels.inferenceDevices, () => core.listComputeDevices())

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
