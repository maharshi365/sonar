import { join } from "node:path"
import { app, BrowserWindow, Menu, shell, type Tray } from "electron"

import { registerIpcHandlers } from "./ipc"
import { closeHistoryStore } from "./history-store"
import { getAppIcon } from "./icon"
import { ensureOverlay } from "./overlay"
import { registerShortcuts, unregisterShortcuts } from "./shortcuts"
import { enableLaunchAtLogin, wasStartedInBackground } from "./startup"
import { refreshSettingsCache } from "./transcription"
import { createTray } from "./tray"

let isQuitting = false
let mainWindow: BrowserWindow | null = null
let tray: Tray | null = null

function hideWindow(): void {
  mainWindow?.hide()
  app.dock?.hide()
}

function createWindow(show = true): BrowserWindow {
  const window = new BrowserWindow({
    title: "Sonar",
    icon: getAppIcon(),
    show,
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

  window.webContents.setWindowOpenHandler(({ url }) => {
    void shell.openExternal(url)
    return { action: "deny" }
  })

  window.on("minimize", hideWindow)

  window.on("close", (event) => {
    if (isQuitting) return
    event.preventDefault()
    hideWindow()
  })

  window.on("closed", () => {
    if (mainWindow === window) mainWindow = null
  })

  if (process.env.ELECTRON_RENDERER_URL) {
    void window.loadURL(process.env.ELECTRON_RENDERER_URL)
  } else {
    void window.loadFile(join(__dirname, "../renderer/index.html"))
  }

  mainWindow = window
  return window
}

function showWindow(): void {
  const window = mainWindow && !mainWindow.isDestroyed() ? mainWindow : createWindow(false)
  if (app.dock) void app.dock.show()
  if (window.isMinimized()) window.restore()
  window.show()
  window.focus()
}

if (!app.requestSingleInstanceLock()) {
  app.quit()
} else {
  app.on("second-instance", () => {
    if (app.isReady()) showWindow()
    else void app.whenReady().then(showWindow)
  })

  void app.whenReady().then(async () => {
    registerIpcHandlers()
    await refreshSettingsCache()
    registerShortcuts(undefined, true)
    Menu.setApplicationMenu(null)

    const launchHidden = wasStartedInBackground()
    createWindow(!launchHidden)
    if (launchHidden) app.dock?.hide()

    tray = createTray(showWindow, () => {
      isQuitting = true
      app.quit()
    })

    void enableLaunchAtLogin().catch((error: unknown) => {
      console.error("Failed to enable launch at login", error)
    })

    // Pre-create the (hidden) dock overlay so it appears instantly on first use.
    ensureOverlay()
    app.on("activate", showWindow)
  })
}

app.on("before-quit", () => {
  isQuitting = true
})

app.on("will-quit", () => {
  unregisterShortcuts()
  closeHistoryStore()
  tray?.destroy()
  tray = null
})
