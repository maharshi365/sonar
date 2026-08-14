/// <reference types="vite/client" />

import type { ModelDownloadProgress, ModelStatus } from "../../shared/models"
import type { Settings } from "../../shared/settings"
import type { StreamText } from "../../shared/transcription"

declare global {
  interface Window {
    sonar: {
      platform: NodeJS.Platform
      settings: {
        get: () => Promise<Settings>
        set: (patch: Partial<Settings>) => Promise<Settings>
      }
      models: {
        list: () => Promise<ModelStatus[]>
        download: (modelId: string) => Promise<void>
        cancel: (modelId: string) => Promise<boolean>
        remove: (modelId: string) => Promise<void>
        onProgress: (
          callback: (progress: ModelDownloadProgress) => void
        ) => () => void
      }
      transcription: {
        toggle: () => Promise<boolean>
        start: () => Promise<void>
        stop: () => Promise<string>
        cancel: () => Promise<void>
        onState: (callback: (recording: boolean) => void) => () => void
        onText: (callback: (text: StreamText) => void) => () => void
        onLevels: (callback: (levels: number[]) => void) => () => void
        onResult: (callback: (text: string) => void) => () => void
        onError: (callback: (message: string) => void) => () => void
      }
    }
  }
}

export {}
