/**
 * IPC channel names shared between the main and preload processes.
 * Kept in one place so both sides can't drift out of sync.
 */
export const IpcChannels = {
  settingsGet: "settings:get",
  settingsSet: "settings:set",

  /** List all catalog models with their on-disk status. */
  modelsList: "models:list",
  /** Start downloading a model by id. */
  modelsDownload: "models:download",
  /** Cancel an in-flight download by id. */
  modelsCancel: "models:cancel",
  /** Remove a downloaded model by id. */
  modelsRemove: "models:remove",

  /**
   * Main -> renderer push channel for download progress. Payload is a
   * `ModelDownloadProgress`. Sent via `webContents.send`.
   */
  modelsProgress: "models:progress",
} as const
