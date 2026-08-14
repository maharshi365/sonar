import { StrictMode, useEffect, useRef, useState } from "react"
import { createRoot } from "react-dom/client"

import type { StreamText } from "../../shared/transcription"
import "@/styles/globals.css"

/** Number of waveform bars — matches the Rust visualizer's bucket count. */
const BAR_COUNT = 16

/**
 * The dock overlay UI.
 *
 * A compact, translucent card pinned near the bottom of the screen while
 * recording. Shows a live audio waveform (driven by level buckets from the Rust
 * pipeline) and the streaming transcript (committed prefix + tentative suffix).
 */
function Overlay() {
  const [text, setText] = useState<StreamText>({ committed: "", tentative: "" })
  const [levels, setLevels] = useState<number[]>(() => new Array(BAR_COUNT).fill(0))
  const scrollRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const offText = window.sonar.transcription.onText(setText)
    const offLevels = window.sonar.transcription.onLevels((next) => {
      setLevels(next.length ? next : new Array(BAR_COUNT).fill(0))
    })
    const offState = window.sonar.transcription.onState((recording) => {
      if (recording) {
        // Reset for a fresh session.
        setText({ committed: "", tentative: "" })
        setLevels(new Array(BAR_COUNT).fill(0))
      }
    })
    return () => {
      offText()
      offLevels()
      offState()
    }
  }, [])

  // Keep the latest text in view as it grows.
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight })
  }, [text])

  const hasText = text.committed.length > 0 || text.tentative.length > 0

  return (
    <div className="flex h-screen w-screen items-end justify-center bg-transparent p-2">
      <div className="flex w-full items-center gap-4 rounded-2xl border border-white/10 bg-[oklch(0.18_0.006_285/0.85)] px-5 py-4 shadow-2xl backdrop-blur-xl">
        {/* Live waveform */}
        <div className="flex h-12 shrink-0 items-center gap-[3px]">
          {levels.map((level, i) => (
            <span
              key={i}
              className="w-[3px] rounded-full bg-primary transition-[height] duration-75"
              style={{
                height: `${Math.max(4, Math.min(48, level * 48))}px`,
                opacity: 0.5 + level * 0.5,
              }}
            />
          ))}
        </div>

        {/* Live transcript */}
        <div
          ref={scrollRef}
          className="max-h-16 flex-1 overflow-y-auto text-left text-sm leading-6 text-foreground [scrollbar-width:none]"
        >
          {hasText ? (
            <p>
              <span>{text.committed}</span>
              <span className="text-muted-foreground">{text.tentative}</span>
            </p>
          ) : (
            <p className="text-muted-foreground">Listening…</p>
          )}
        </div>
      </div>
    </div>
  )
}

createRoot(document.getElementById("overlay-root")!).render(
  <StrictMode>
    <Overlay />
  </StrictMode>,
)
