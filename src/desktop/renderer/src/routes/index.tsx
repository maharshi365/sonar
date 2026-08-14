import { createFileRoute } from "@tanstack/react-router"
import { useEffect, useState } from "react"
import { Loader2, Mic, ShieldCheck, Square, Waves } from "lucide-react"

import { Button } from "@/components/ui/button"
import type { TranscriptionState } from "../../../shared/transcription"

export const Route = createFileRoute("/")({
  component: HomePage,
})

function HomePage() {
  const [state, setState] = useState<TranscriptionState>("idle")
  const [transcript, setTranscript] = useState("")
  const [live, setLive] = useState("")
  const [error, setError] = useState("")
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    const offState = window.sonar.transcription.onState((nextState) => {
      setState(nextState)
      if (nextState === "recording") {
        setLive("")
        setTranscript("")
        setError("")
      }
      setBusy(nextState === "transcribing")
    })
    const offText = window.sonar.transcription.onText((text) => {
      setLive(text.committed + text.tentative)
    })
    const offResult = window.sonar.transcription.onResult((text) => {
      setTranscript(text)
      setLive("")
      setBusy(false)
    })
    const offError = window.sonar.transcription.onError((message) => {
      setError(message)
      setBusy(false)
    })
    return () => {
      offState()
      offText()
      offResult()
      offError()
    }
  }, [])

  const recording = state === "recording"
  const transcribing = state === "transcribing"

  const toggle = async () => {
    setError("")
    setBusy(true)
    await window.sonar.transcription.toggle()
  }

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header>
        <p className="mb-2 text-xs font-medium uppercase tracking-[0.2em] text-primary">Local transcription</p>
        <h1 className="text-3xl font-semibold tracking-tight">Ready when you are.</h1>
        <p className="mt-2 max-w-xl text-sm leading-6 text-muted-foreground">
          Press the button or the global shortcut and Sonar will type the transcript into your active app.
        </p>
      </header>

      <section className="flex flex-1 flex-col items-center justify-center py-12 text-center">
        <div className="relative mb-8">
          <div
            className={`absolute inset-0 scale-150 rounded-full blur-2xl transition-colors ${recording ? "bg-destructive/20" : "bg-primary/10"}`}
          />
          <Button
            className="relative size-24 rounded-full shadow-[0_0_50px_-12px_var(--primary)]"
            size="icon"
            aria-label={
              recording
                ? "Stop recording"
                : transcribing
                  ? "Transcribing"
                  : "Start recording"
            }
            variant={recording ? "destructive" : "default"}
            onClick={toggle}
            disabled={busy}
          >
            {busy ? (
              <Loader2 className="size-8 animate-spin" />
            ) : recording ? (
              <Square className="size-7" />
            ) : (
              <Mic className="size-8" />
            )}
          </Button>
        </div>
        <h2 className="text-lg font-medium">
          {recording
            ? "Listening… press to stop"
            : transcribing
              ? "Transcribing…"
              : "Press to start speaking"}
        </h2>
        <div className="mt-3 flex items-center gap-2 text-xs text-muted-foreground">
          <kbd className="rounded border border-border bg-secondary px-2 py-1 font-mono text-foreground">Ctrl</kbd>
          <span>+</span>
          <kbd className="rounded border border-border bg-secondary px-2 py-1 font-mono text-foreground">Shift</kbd>
          <span>+</span>
          <kbd className="rounded border border-border bg-secondary px-2 py-1 font-mono text-foreground">Space</kbd>
        </div>

        {error && (
          <p className="mt-6 max-w-xl rounded-lg border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive">
            {error}
          </p>
        )}

        {(live || transcript) && (
          <div className="mt-8 w-full max-w-xl rounded-xl border border-border bg-card px-5 py-4 text-left">
            <p className="mb-1 text-xs font-medium uppercase tracking-wider text-muted-foreground">
              {recording ? "Live" : "Transcript"}
            </p>
            <p className="text-sm leading-6 text-foreground">{live || transcript}</p>
          </div>
        )}
      </section>

      <footer className="mt-auto flex flex-wrap items-center gap-x-6 gap-y-2 border-t border-border pt-5 text-xs text-muted-foreground">
        <span className="flex items-center gap-2"><Waves className="size-3.5 text-primary" /> Whisper (whisper.cpp)</span>
        <span className="flex items-center gap-2"><ShieldCheck className="size-3.5" /> Audio stays on this device</span>
      </footer>
    </div>
  )
}
