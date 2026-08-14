/**
 * Shared model types for Sonar.
 *
 * These mirror the shapes produced by the Rust core (`@sonar/core`) but live in
 * `shared` so both the main process and the renderer can import them without
 * pulling in the native addon (which only the main process may load).
 */

/** On-disk status of a catalog model. Mirrors Rust `JsModelStatus`. */
export interface ModelStatus {
  id: string
  name: string
  description: string
  filename: string
  sizeBytes: number
  languages: string[]
  supportsStreaming: boolean
  recommended: boolean
  isDownloaded: boolean
  isDownloading: boolean
  partialBytes: number
}

/** Progress event for an in-flight download. Mirrors Rust `JsDownloadProgress`. */
export interface ModelDownloadProgress {
  modelId: string
  downloaded: number
  total: number
  percentage: number
}
