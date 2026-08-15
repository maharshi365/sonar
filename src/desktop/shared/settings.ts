import { z } from "zod"

const generalSettingsSchema = z.object({
  ttsModel: z.string().default(""),
  modelUnloadTimeout: z
    .enum(["never", "immediately", "2m", "5m", "10m", "15m", "1h"])
    .default("5m"),
  historyLimit: z.number().int().min(0).max(10_000).default(100),
})

const shortcutSettingsSchema = z.object({
  transcribe: z.string().min(1).default("CommandOrControl+Shift+Space"),
  cancel: z.string().min(1).default("CommandOrControl+Shift+Backspace"),
})

const audioSettingsSchema = z.object({
  inputDeviceId: z.string().default(""),
})

const outputSettingsSchema = z.object({
  method: z.enum(["paste", "clipboard", "none"]).default("paste"),
  appendTrailingSpace: z.boolean().default(false),
  autoSubmit: z.boolean().default(false),
  autoSubmitKey: z.enum(["enter", "ctrl_enter", "cmd_enter"]).default("enter"),
})

const transcriptionSettingsSchema = z.object({
  customWords: z.array(z.string()).default([]),
  fillerWordRemoval: z.boolean().default(true),
  customFillerWords: z.array(z.string()).default([]),
  extraRecordingBufferMs: z.number().int().min(0).max(5_000).default(0),
  wordCorrectionThreshold: z.number().min(0).max(1).default(0.18),
})

const inferenceSettingsSchema = z.object({
  accelerator: z.enum(["auto", "cpu", "gpu"]).default("auto"),
  gpuDeviceId: z.string().default(""),
})

const authSettingsSchema = z.object({
  huggingFaceToken: z.string().default(""),
})

/** Root settings object persisted to settings.json. */
export const settingsSchema = z.object({
  schemaVersion: z.number().int().default(1),
  general: generalSettingsSchema.prefault({}),
  shortcuts: shortcutSettingsSchema.prefault({}),
  audio: audioSettingsSchema.prefault({}),
  output: outputSettingsSchema.prefault({}),
  transcription: transcriptionSettingsSchema.prefault({}),
  inference: inferenceSettingsSchema.prefault({}),
  auth: authSettingsSchema.prefault({}),
})

export type Settings = z.infer<typeof settingsSchema>
export type GeneralSettings = Settings["general"]
export type ShortcutSettings = Settings["shortcuts"]
export type AudioSettings = Settings["audio"]
export type OutputSettings = Settings["output"]
export type TranscriptionSettings = Settings["transcription"]
export type InferenceSettings = Settings["inference"]
export type AuthSettings = Settings["auth"]
export type SettingsPatch = {
  schemaVersion?: number
  general?: Partial<GeneralSettings>
  shortcuts?: Partial<ShortcutSettings>
  audio?: Partial<AudioSettings>
  output?: Partial<OutputSettings>
  transcription?: Partial<TranscriptionSettings>
  inference?: Partial<InferenceSettings>
  auth?: Partial<AuthSettings>
}

export const defaultSettings: Settings = settingsSchema.parse({})

export function parseSettings(input: Record<string, unknown> = {}): Settings {
  return settingsSchema.parse(input)
}
