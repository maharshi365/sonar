import { ipcMain } from "electron"

import { IpcChannels } from "../shared/ipc"
import {
  cancelDownload,
  downloadModel,
  listModels,
  removeModel,
} from "./models"
import { loadSettings, saveSettings } from "./settings-store"

/**
 * Register all main-process IPC handlers. Called once during app startup.
 */
export function registerIpcHandlers(): void {
  ipcMain.handle(IpcChannels.settingsGet, () => loadSettings())
  ipcMain.handle(IpcChannels.settingsSet, (_event, patch: unknown) =>
    saveSettings(patch)
  )

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
}
