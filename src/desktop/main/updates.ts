import { app, BrowserWindow } from "electron"
import { autoUpdater } from "electron-updater"

import { IpcChannels } from "../shared/ipc"
import type { UpdateStatus } from "../shared/updates"

let configured = false
let status: UpdateStatus = {
  currentVersion: app.getVersion(),
  phase: app.isPackaged ? "idle" : "unsupported",
  message: app.isPackaged ? undefined : "Update checks are available in installed builds.",
}

export function getUpdateStatus(): UpdateStatus {
  configureUpdater()
  return status
}

export async function checkForUpdates(): Promise<UpdateStatus> {
  configureUpdater()
  if (!app.isPackaged) return status

  setStatus({ phase: "checking" })
  try {
    await autoUpdater.checkForUpdates()
  } catch (error) {
    setStatus({ phase: "error", message: errorMessage(error) })
  }
  return status
}

export function installUpdate(): void {
  configureUpdater()
  if (status.phase !== "downloaded") {
    throw new Error("No update is ready to install")
  }
  autoUpdater.quitAndInstall(false, true)
}

function configureUpdater(): void {
  if (configured || !app.isPackaged) return
  configured = true

  autoUpdater.autoDownload = true
  autoUpdater.autoInstallOnAppQuit = false

  autoUpdater.on("checking-for-update", () => setStatus({ phase: "checking" }))
  autoUpdater.on("update-not-available", () => setStatus({ phase: "up-to-date" }))
  autoUpdater.on("update-available", (info) =>
    setStatus({ phase: "available", version: info.version })
  )
  autoUpdater.on("download-progress", (progress) =>
    setStatus({
      phase: "downloading",
      version: status.version,
      percent: Math.round(progress.percent),
    })
  )
  autoUpdater.on("update-downloaded", (info) =>
    setStatus({ phase: "downloaded", version: info.version, percent: 100 })
  )
  autoUpdater.on("error", (error) =>
    setStatus({ phase: "error", message: errorMessage(error) })
  )
}

function setStatus(next: Omit<UpdateStatus, "currentVersion">): void {
  status = { currentVersion: app.getVersion(), ...next }
  for (const window of BrowserWindow.getAllWindows()) {
    if (!window.isDestroyed()) {
      window.webContents.send(IpcChannels.updatesStatus, status)
    }
  }
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "The update check failed."
}
