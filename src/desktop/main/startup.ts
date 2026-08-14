import { mkdir, writeFile } from "node:fs/promises"
import { join } from "node:path"
import { app } from "electron"

const HIDDEN_LAUNCH_ARGUMENT = "--hidden"

export function wasStartedInBackground(): boolean {
  return (
    process.argv.includes(HIDDEN_LAUNCH_ARGUMENT) ||
    (process.platform === "darwin" && app.getLoginItemSettings().wasOpenedAtLogin)
  )
}

export async function enableLaunchAtLogin(): Promise<void> {
  if (!app.isPackaged) return

  if (process.platform !== "linux") {
    app.setLoginItemSettings({
      openAtLogin: true,
      openAsHidden: true,
      args: [HIDDEN_LAUNCH_ARGUMENT],
    })
    return
  }

  const autostartDirectory = join(app.getPath("home"), ".config", "autostart")
  const executable = process.execPath.replaceAll("\\", "\\\\").replaceAll('"', '\\"')
  const desktopEntry = `[Desktop Entry]
Type=Application
Name=Sonar
Comment=Local-first desktop speech-to-text
Exec="${executable}" ${HIDDEN_LAUNCH_ARGUMENT}
Terminal=false
StartupNotify=false
X-GNOME-Autostart-enabled=true
`

  await mkdir(autostartDirectory, { recursive: true })
  await writeFile(join(autostartDirectory, "com.sonar.desktop.desktop"), desktopEntry, "utf8")
}
