import { Loader2, Trash2 } from "lucide-react"

import { Button } from "@/components/ui/button"

interface HistoryHeaderProps {
  isClearing?: boolean
  onClear?: () => void
}

export function HistoryHeader({ isClearing, onClear }: HistoryHeaderProps) {
  return (
    <header className="flex flex-wrap items-end justify-between gap-5">
      <div>
        <p className="mb-2 text-xs font-medium uppercase tracking-[0.2em] text-primary">
          Stored on this device
        </p>
        <h1 className="text-3xl font-semibold tracking-tight">Transcription history</h1>
        <p className="mt-2 max-w-xl text-sm leading-6 text-muted-foreground">
          Revisit and copy your completed transcriptions. Nothing leaves your device.
        </p>
      </div>

      {onClear ? (
        <Button
          variant="ghost"
          size="sm"
          className="text-muted-foreground hover:text-destructive"
          disabled={isClearing}
          onClick={onClear}
        >
          {isClearing ? <Loader2 className="animate-spin" /> : <Trash2 />}
          Clear history
        </Button>
      ) : null}
    </header>
  )
}
