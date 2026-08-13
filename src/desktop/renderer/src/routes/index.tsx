import { createFileRoute } from "@tanstack/react-router"
import { Mic, ShieldCheck, Waves } from "lucide-react"

import { Button } from "@/components/ui/button"

export const Route = createFileRoute("/")({
  component: HomePage,
})

function HomePage() {
  return (
    <div className="mx-auto flex min-h-full max-w-4xl flex-col px-8 py-10 lg:px-12">
      <header>
        <p className="mb-2 text-xs font-medium uppercase tracking-[0.2em] text-primary">Local transcription</p>
        <h1 className="text-3xl font-semibold tracking-tight">Ready when you are.</h1>
        <p className="mt-2 max-w-xl text-sm leading-6 text-muted-foreground">
          Start a recording and Sonar will type the transcript into your active app.
        </p>
      </header>

      <section className="my-auto flex flex-col items-center py-14 text-center">
        <div className="relative mb-8">
          <div className="absolute inset-0 scale-150 rounded-full bg-primary/10 blur-2xl" />
          <Button className="relative size-24 rounded-full shadow-[0_0_50px_-12px_var(--primary)]" size="icon" aria-label="Start recording">
            <Mic className="size-8" />
          </Button>
        </div>
        <h2 className="text-lg font-medium">Press to start speaking</h2>
        <div className="mt-3 flex items-center gap-2 text-xs text-muted-foreground">
          <kbd className="rounded border border-border bg-secondary px-2 py-1 font-mono text-foreground">Ctrl</kbd>
          <span>+</span>
          <kbd className="rounded border border-border bg-secondary px-2 py-1 font-mono text-foreground">Shift</kbd>
          <span>+</span>
          <kbd className="rounded border border-border bg-secondary px-2 py-1 font-mono text-foreground">Space</kbd>
        </div>
      </section>

      <footer className="flex flex-wrap items-center gap-x-6 gap-y-2 border-t border-border pt-5 text-xs text-muted-foreground">
        <span className="flex items-center gap-2"><Waves className="size-3.5 text-primary" /> No model installed</span>
        <span className="flex items-center gap-2"><ShieldCheck className="size-3.5" /> Audio stays on this device</span>
      </footer>
    </div>
  )
}
