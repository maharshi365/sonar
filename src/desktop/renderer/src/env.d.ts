/// <reference types="vite/client" />

import type { Settings } from "../../shared/settings"

declare global {
  interface Window {
    sonar: {
      platform: NodeJS.Platform
      settings: {
        get: () => Promise<Settings>
        set: (patch: Partial<Settings>) => Promise<Settings>
      }
    }
  }
}

export {}
