import { Menu, Tray } from "electron"

import { getAppIcon } from "./icon"

export function createTray(showWindow: () => void, quit: () => void): Tray {
  const tray = new Tray(getAppIcon().resize({ width: 16, height: 16 }))
  tray.setToolTip("Sonar")
  tray.setContextMenu(
    Menu.buildFromTemplate([
      { label: "Open Sonar", click: showWindow },
      { type: "separator" },
      { label: "Quit Sonar", click: quit },
    ]),
  )
  tray.on("click", showWindow)
  return tray
}
