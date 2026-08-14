import { join } from "node:path"
import { app, BrowserWindow, Menu, shell } from "electron"

import { registerIpcHandlers } from "./ipc"
import { ensureOverlay } from "./overlay"
import { registerShortcuts, unregisterShortcuts } from "./shortcuts"
import { refreshSettingsCache } from "./transcription"

function createWindow(): void {
  const window = new BrowserWindow({
    title: "Sonar",
    width: 1120,
    height: 760,
    minWidth: 760,
    minHeight: 560,
    backgroundColor: "#121217",
    webPreferences: {
      preload: join(__dirname, "../preload/index.js"),
      sandbox: true,
      contextIsolation: true,
      nodeIntegration: false,
    },
  })

  Menu.setApplicationMenu(null)

  window.webContents.setWindowOpenHandler(({ url }) => {
    void shell.openExternal(url)
    return { action: "deny" }
  })

  if (process.env.ELECTRON_RENDERER_URL) {
    void window.loadURL(process.env.ELECTRON_RENDERER_URL)
  } else {
    void window.loadFile(join(__dirname, "../renderer/index.html"))
  }
}

void app.whenReady().then(() => {
  registerIpcHandlers()
  registerShortcuts()
  createWindow()
  // Pre-create the (hidden) dock overlay so it appears instantly on first use.
  ensureOverlay()
  // Warm the settings cache so the first recording sees the selected model.
  void refreshSettingsCache()

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
})

app.on("will-quit", () => {
  unregisterShortcuts()
})

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") app.quit()
})
