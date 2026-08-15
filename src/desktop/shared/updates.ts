export type UpdatePhase =
  | "idle"
  | "checking"
  | "up-to-date"
  | "available"
  | "downloading"
  | "downloaded"
  | "error"
  | "unsupported"

export interface UpdateStatus {
  currentVersion: string
  phase: UpdatePhase
  version?: string
  percent?: number
  message?: string
}
