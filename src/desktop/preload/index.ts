import { contextBridge, ipcRenderer, type IpcRendererEvent } from "electron"

import { IpcChannels } from "../shared/ipc"
import type { ModelDownloadProgress, ModelStatus } from "../shared/models"
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
  models: {
    /** List all catalog models with their on-disk status. */
    list: (): Promise<ModelStatus[]> =>
      ipcRenderer.invoke(IpcChannels.modelsList),
    /** Start (or resume) downloading a model. Resolves when complete. */
    download: (modelId: string): Promise<void> =>
      ipcRenderer.invoke(IpcChannels.modelsDownload, modelId),
    /** Cancel an in-flight download. Resolves to whether one was running. */
    cancel: (modelId: string): Promise<boolean> =>
      ipcRenderer.invoke(IpcChannels.modelsCancel, modelId),
    /** Remove a downloaded model from disk. */
    remove: (modelId: string): Promise<void> =>
      ipcRenderer.invoke(IpcChannels.modelsRemove, modelId),
    /**
     * Subscribe to download-progress events. Returns an unsubscribe function.
     */
    onProgress: (
      callback: (progress: ModelDownloadProgress) => void
    ): (() => void) => {
      const listener = (
        _event: IpcRendererEvent,
        progress: ModelDownloadProgress
      ): void => callback(progress)
      ipcRenderer.on(IpcChannels.modelsProgress, listener)
      return () => ipcRenderer.removeListener(IpcChannels.modelsProgress, listener)
    },
  },
})
