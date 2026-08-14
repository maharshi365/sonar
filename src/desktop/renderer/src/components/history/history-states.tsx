import { History } from "lucide-react"

import { Skeleton } from "@/components/ui/skeleton"

export function HistoryLoading() {
  return (
    <div className="mt-8 space-y-3">
      <Skeleton className="h-36 w-full rounded-xl" />
      <Skeleton className="h-32 w-full rounded-xl" />
      <Skeleton className="h-40 w-full rounded-xl" />
    </div>
  )
}

export function HistoryEmpty() {
  return (
    <div className="mt-8 flex min-h-80 flex-col items-center justify-center border border-border bg-card/20 px-6 text-center">
      <div className="mb-4 flex size-11 items-center justify-center rounded-full bg-primary/10 text-primary">
        <History className="size-5" />
      </div>
      <h2 className="font-medium">No transcription history</h2>
      <p className="mt-2 max-w-sm text-sm leading-6 text-muted-foreground">
        Completed transcriptions will be saved locally and appear here.
      </p>
    </div>
  )
}

export function HistoryLoadError({ message }: { message: string }) {
  return (
    <div className="mt-8 border border-destructive/30 bg-destructive/10 p-5 text-sm text-destructive">
      Failed to load transcription history: {message}
    </div>
  )
}

export function HistoryUpdateError() {
  return (
    <p className="mt-6 border border-destructive/30 bg-destructive/10 p-4 text-sm text-destructive">
      Failed to update transcription history. Please try again.
    </p>
  )
}
