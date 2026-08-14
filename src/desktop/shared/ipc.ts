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

  // --- Live transcription ---
  /** Toggle recording on/off (renderer -> main). Resolves to the new state. */
  transcriptionToggle: "transcription:toggle",
  /** Start recording (renderer -> main). */
  transcriptionStart: "transcription:start",
  /** Stop recording and transcribe (renderer -> main). Resolves to text. */
  transcriptionStop: "transcription:stop",
  /** Cancel an in-flight recording (renderer -> main). */
  transcriptionCancel: "transcription:cancel",

  /** Main -> renderer: recording state changed. Payload: boolean. */
  transcriptionState: "transcription:state",
  /** Main -> renderer: live text update. Payload: { committed, tentative }. */
  transcriptionText: "transcription:text",
  /** Main -> renderer: audio level buckets. Payload: number[]. */
  transcriptionLevels: "transcription:levels",
  /** Main -> renderer: a final transcript is ready. Payload: string. */
  transcriptionResult: "transcription:result",
  /** Main -> renderer: an error occurred. Payload: string. */
  transcriptionError: "transcription:error",
} as const
