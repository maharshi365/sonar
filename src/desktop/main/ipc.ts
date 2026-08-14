import { ipcMain } from "electron"

import { IpcChannels } from "../shared/ipc"
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

  // Live transcription.
  ipcMain.handle(IpcChannels.transcriptionToggle, () => toggleRecording())
  ipcMain.handle(IpcChannels.transcriptionStart, () => startRecording())
  ipcMain.handle(IpcChannels.transcriptionStop, () => stopRecording())
  ipcMain.handle(IpcChannels.transcriptionCancel, () => cancelRecording())
}
