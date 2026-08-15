import { contextBridge, ipcRenderer, type IpcRendererEvent } from "electron"

import { IpcChannels } from "../shared/ipc"
import type { AudioInputDevice, ComputeDevice } from "../shared/devices"
import type { HistoryPage } from "../shared/history"
import type { ModelDownloadProgress, ModelStatus } from "../shared/models"
import type { Settings, SettingsPatch } from "../shared/settings"
import type { StreamText, TranscriptionState } from "../shared/transcription"
import type { UpdateStatus } from "../shared/updates"

contextBridge.exposeInMainWorld("sonar", {
  platform: process.platform,
  settings: {
    /** Read the current, fully-validated settings from the main process. */
    get: (): Promise<Settings> => ipcRenderer.invoke(IpcChannels.settingsGet),
    /**
     * Persist a (possibly partial) settings patch. Returns the resulting
     * validated settings object.
     */
    set: (patch: SettingsPatch): Promise<Settings> =>
      ipcRenderer.invoke(IpcChannels.settingsSet, patch),
  },
  devices: {
    inputs: (): Promise<AudioInputDevice[]> =>
      ipcRenderer.invoke(IpcChannels.audioInputDevices),
    compute: (): Promise<ComputeDevice[]> =>
      ipcRenderer.invoke(IpcChannels.inferenceDevices),
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
  history: {
    list: (cursor?: number, limit?: number): Promise<HistoryPage> =>
      ipcRenderer.invoke(IpcChannels.historyList, cursor, limit),
    delete: (id: number): Promise<boolean> =>
      ipcRenderer.invoke(IpcChannels.historyDelete, id),
    clear: (): Promise<void> => ipcRenderer.invoke(IpcChannels.historyClear),
    onChanged: (callback: () => void): (() => void) => {
      const listener = (): void => callback()
      ipcRenderer.on(IpcChannels.historyChanged, listener)
      return () => ipcRenderer.removeListener(IpcChannels.historyChanged, listener)
    },
  },
  updates: {
    getStatus: (): Promise<UpdateStatus> =>
      ipcRenderer.invoke(IpcChannels.updatesGetStatus),
    check: (): Promise<UpdateStatus> =>
      ipcRenderer.invoke(IpcChannels.updatesCheck),
    install: (): Promise<void> => ipcRenderer.invoke(IpcChannels.updatesInstall),
    onStatus: (callback: (status: UpdateStatus) => void): (() => void) => {
      const listener = (_event: IpcRendererEvent, status: UpdateStatus): void =>
        callback(status)
      ipcRenderer.on(IpcChannels.updatesStatus, listener)
      return () => ipcRenderer.removeListener(IpcChannels.updatesStatus, listener)
    },
  },
  transcription: {
    /** Toggle recording on/off. Resolves to the resulting state. */
    toggle: (): Promise<boolean> =>
      ipcRenderer.invoke(IpcChannels.transcriptionToggle),
    /** Start recording. */
    start: (): Promise<void> =>
      ipcRenderer.invoke(IpcChannels.transcriptionStart),
    /** Stop recording and transcribe. Resolves to the final text. */
    stop: (): Promise<string> =>
      ipcRenderer.invoke(IpcChannels.transcriptionStop),
    /** Cancel an in-flight recording. */
    cancel: (): Promise<void> =>
      ipcRenderer.invoke(IpcChannels.transcriptionCancel),

    /** Subscribe to lifecycle state changes. Returns an unsubscribe fn. */
    onState: (callback: (state: TranscriptionState) => void): (() => void) => {
      const listener = (_e: IpcRendererEvent, state: TranscriptionState): void =>
        callback(state)
      ipcRenderer.on(IpcChannels.transcriptionState, listener)
      return () =>
        ipcRenderer.removeListener(IpcChannels.transcriptionState, listener)
    },
    /** Subscribe to live text updates. Returns an unsubscribe fn. */
    onText: (callback: (text: StreamText) => void): (() => void) => {
      const listener = (_e: IpcRendererEvent, text: StreamText): void =>
        callback(text)
      ipcRenderer.on(IpcChannels.transcriptionText, listener)
      return () =>
        ipcRenderer.removeListener(IpcChannels.transcriptionText, listener)
    },
    /** Subscribe to audio level buckets (0..1). Returns an unsubscribe fn. */
    onLevels: (callback: (levels: number[]) => void): (() => void) => {
      const listener = (_e: IpcRendererEvent, levels: number[]): void =>
        callback(levels)
      ipcRenderer.on(IpcChannels.transcriptionLevels, listener)
      return () =>
        ipcRenderer.removeListener(IpcChannels.transcriptionLevels, listener)
    },
    /** Subscribe to final transcript results. Returns an unsubscribe fn. */
    onResult: (callback: (text: string) => void): (() => void) => {
      const listener = (_e: IpcRendererEvent, text: string): void =>
        callback(text)
      ipcRenderer.on(IpcChannels.transcriptionResult, listener)
      return () =>
        ipcRenderer.removeListener(IpcChannels.transcriptionResult, listener)
    },
    /** Subscribe to transcription errors. Returns an unsubscribe fn. */
    onError: (callback: (message: string) => void): (() => void) => {
      const listener = (_e: IpcRendererEvent, message: string): void =>
        callback(message)
      ipcRenderer.on(IpcChannels.transcriptionError, listener)
      return () =>
        ipcRenderer.removeListener(IpcChannels.transcriptionError, listener)
    },
  },
})
