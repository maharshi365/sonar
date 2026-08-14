import { join } from "node:path"
import { BrowserWindow, screen } from "electron"

/**
 * The dock overlay: a small, frameless, always-on-top window that appears near
 * the bottom of the screen while recording, showing live transcription text and
 * an audio waveform. Created lazily and reused; hidden (not destroyed) between
 * sessions so it can reappear instantly.
 */

const OVERLAY_WIDTH = 520
const OVERLAY_HEIGHT = 140
/** Gap between the overlay and the bottom edge of the work area. */
const BOTTOM_MARGIN = 80

let overlay: BrowserWindow | null = null

/** Position the overlay centered horizontally, near the bottom of the display. */
function positionOverlay(window: BrowserWindow): void {
  const { workArea } = screen.getPrimaryDisplay()
  const x = Math.round(workArea.x + (workArea.width - OVERLAY_WIDTH) / 2)
  const y = Math.round(workArea.y + workArea.height - OVERLAY_HEIGHT - BOTTOM_MARGIN)
  window.setPosition(x, y)
}

/** Create the overlay window (hidden). Idempotent. */
export function ensureOverlay(): BrowserWindow {
  if (overlay && !overlay.isDestroyed()) return overlay

  overlay = new BrowserWindow({
    width: OVERLAY_WIDTH,
    height: OVERLAY_HEIGHT,
    show: false,
    frame: false,
    transparent: true,
    resizable: false,
    movable: false,
    minimizable: false,
    maximizable: false,
    fullscreenable: false,
    skipTaskbar: true,
    alwaysOnTop: true,
    focusable: false,
    hasShadow: false,
    backgroundColor: "#00000000",
    webPreferences: {
      preload: join(__dirname, "../preload/index.js"),
      sandbox: true,
      contextIsolation: true,
      nodeIntegration: false,
    },
  })

  // Float above full-screen apps and other always-on-top windows.
  overlay.setAlwaysOnTop(true, "screen-saver")
  overlay.setVisibleOnAllWorkspaces(true, { visibleOnFullScreen: true })

  const query = "?window=overlay"
  if (process.env.ELECTRON_RENDERER_URL) {
    void overlay.loadURL(`${process.env.ELECTRON_RENDERER_URL}/overlay.html${query}`)
  } else {
    void overlay.loadFile(join(__dirname, "../renderer/overlay.html"), {
      search: "window=overlay",
    })
  }

  return overlay
}

/** Show the overlay, repositioning it for the current display. */
export function showOverlay(): void {
  const window = ensureOverlay()
  positionOverlay(window)
  window.showInactive()
}

/** Hide the overlay without destroying it. */
export function hideOverlay(): void {
  if (overlay && !overlay.isDestroyed()) overlay.hide()
}

/** Send an IPC message to the overlay window, if it exists. */
export function sendToOverlay(channel: string, ...args: unknown[]): void {
  if (overlay && !overlay.isDestroyed()) {
    overlay.webContents.send(channel, ...args)
  }
}
