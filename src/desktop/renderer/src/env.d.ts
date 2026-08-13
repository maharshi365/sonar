/// <reference types="vite/client" />

import type { ModelDownloadProgress, ModelStatus } from "../../shared/models"
import type { Settings } from "../../shared/settings"

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
    }
  }
}

export {}
