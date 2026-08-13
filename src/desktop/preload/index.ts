import { contextBridge, ipcRenderer } from "electron"

import { IpcChannels } from "../shared/ipc"
import type { Settings } from "../shared/settings"

contextBridge.exposeInMainWorld("sonar", {
  platform: process.platform,
  settings: {
    /** Read the current, fully-validated settings from the main process. */
    get: (): Promise<Settings> => ipcRenderer.invoke(IpcChannels.settingsGet),
    /**
     * Persist a (possibly partial) settings patch. Returns the resulting
     * validated settings object.
     */
    set: (patch: Partial<Settings>): Promise<Settings> =>
      ipcRenderer.invoke(IpcChannels.settingsSet, patch),
  },
})
