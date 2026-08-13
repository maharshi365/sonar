/**
 * IPC channel names shared between the main and preload processes.
 * Kept in one place so both sides can't drift out of sync.
 */
export const IpcChannels = {
  settingsGet: "settings:get",
  settingsSet: "settings:set",
} as const
