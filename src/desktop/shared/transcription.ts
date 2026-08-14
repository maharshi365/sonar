/**
 * Shared live-transcription types.
 *
 * Live in `shared` so both the main process and the renderer(s) can import them
 * without pulling in the native addon (main-process only).
 */

/** A live text snapshot emitted while recording. Mirrors Rust `JsStreamText`. */
export interface StreamText {
  /** Append-only, flicker-free prefix. */
  committed: string
  /** Volatile suffix the model may still rewrite. */
  tentative: string
}

/** Recording lifecycle state pushed to renderers. */
export type TranscriptionState = boolean
