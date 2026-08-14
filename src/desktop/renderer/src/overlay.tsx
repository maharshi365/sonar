import { ChevronUp, Minimize2 } from "lucide-react"
import { StrictMode, useEffect, useRef, useState } from "react"
import { createRoot } from "react-dom/client"

import type { StreamText } from "../../shared/transcription"
import "@/styles/globals.css"
import "@/styles/overlay.css"

const BAR_COUNT = 16

function Waveform({ levels, previewing }: { levels: number[]; previewing: boolean }) {
  return (
    <div
      className={`overlay-waveform ${previewing ? "is-previewing" : ""}`}
      aria-hidden="true"
    >
      {levels.map((level, index) => (
        <span
          key={index}
          style={{
            height: `${Math.max(3, Math.min(24, level * 24))}px`,
            opacity: 0.48 + level * 0.52,
          }}
        />
      ))}
    </div>
  )
}

function Transcript({ text }: { text: StreamText }) {
  return (
    <p>
      <span>{text.committed}</span>
      <span className="overlay-tentative">{text.tentative}</span>
    </p>
  )
}

function TranscriptTail({ text }: { text: StreamText }) {
  const fullText = `${text.committed}${text.tentative}`
  const slicedTail = fullText.slice(-96)
  const firstSpace = slicedTail.indexOf(" ")
  const tail = fullText.length > slicedTail.length && firstSpace >= 0
    ? slicedTail.slice(firstSpace + 1)
    : slicedTail
  return <p>{fullText.length > tail.length ? `...${tail}` : tail}</p>
}

function Overlay() {
  const [text, setText] = useState<StreamText>({ committed: "", tentative: "" })
  const [levels, setLevels] = useState<number[]>(() => new Array(BAR_COUNT).fill(0))
  const [expanded, setExpanded] = useState(false)
  const scrollRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const offText = window.sonar.transcription.onText(setText)
    const offLevels = window.sonar.transcription.onLevels((next) => {
      setLevels(next.length ? next : new Array(BAR_COUNT).fill(0))
    })
    const offState = window.sonar.transcription.onState((recording) => {
      if (recording) {
        setText({ committed: "", tentative: "" })
        setLevels(new Array(BAR_COUNT).fill(0))
        setExpanded(false)
      }
    })
    return () => {
      offText()
      offLevels()
      offState()
    }
  }, [])

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight })
  }, [text])

  const displayText = text.committed || text.tentative
    ? text
    : { committed: "Listening...", tentative: "" }
  const previewingLevels = !levels.some(Boolean)
  const displayLevels = !previewingLevels
    ? levels
    : levels.map((_, index) => 0.18 + ((index * 7) % 9) / 13)

  return (
    <main className="overlay-stage">
      <section
        className={`overlay-dock overlay-dock--rail ${expanded ? "is-expanded" : ""}`}
      >
        <div className="overlay-expanded-panel" aria-hidden={!expanded}>
          <header>
            <div>
              <span className="overlay-eyebrow">Live transcript</span>
              <span className="overlay-status"><i /> Recording</span>
            </div>
            <button
              type="button"
              className="overlay-icon-button"
              aria-label="Collapse transcript"
              onClick={() => setExpanded(false)}
            >
              <Minimize2 size={15} strokeWidth={1.8} />
            </button>
          </header>
          <div ref={scrollRef} className="overlay-full-transcript">
            <Transcript text={displayText} />
          </div>
        </div>

        <div className="overlay-compact-row">
          <span className="overlay-recording-dot" aria-label="Recording" />
          <Waveform levels={displayLevels} previewing={previewingLevels} />
          <div className="overlay-peek" aria-hidden={expanded}>
            <TranscriptTail text={displayText} />
          </div>
          <button
            type="button"
            className="overlay-expand-button"
            aria-label="Expand transcript"
            onClick={() => setExpanded(true)}
          >
            <ChevronUp size={16} strokeWidth={2} />
          </button>
        </div>
      </section>
    </main>
  )
}

createRoot(document.getElementById("overlay-root")!).render(
  <StrictMode>
    <Overlay />
  </StrictMode>,
)
