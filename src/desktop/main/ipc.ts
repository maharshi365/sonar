import { ipcMain } from "electron"

import { IpcChannels } from "../shared/ipc"
import { loadSettings, saveSettings } from "./settings-store"

/**
 * Register all main-process IPC handlers. Called once during app startup.
 */
export function registerIpcHandlers(): void {
  ipcMain.handle(IpcChannels.settingsGet, () => loadSettings())
  ipcMain.handle(IpcChannels.settingsSet, (_event, patch: unknown) =>
    saveSettings(patch)
  )
}
