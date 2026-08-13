import { Check, Download, Loader2, Trash2, X } from "lucide-react"

import type { ModelStatus } from "../../../../shared/models"
import type { ModelProgress } from "@/hooks/use-models"
import { Button } from "@/components/ui/button"
import { Progress } from "@/components/ui/progress"

function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 MB"
  const gb = bytes / 1e9
  if (gb >= 1) return `${gb.toFixed(1)} GB`
  return `${Math.round(bytes / 1e6)} MB`
}

function formatSpeed(bytesPerSecond: number): string {
  if (bytesPerSecond <= 0) return ""
  const mbps = bytesPerSecond / 1e6
  return `${mbps.toFixed(1)} MB/s`
}

export function ModelCard({
  model,
  progress,
  error,
  onDownload,
  onCancel,
  onRemove,
}: {
  model: ModelStatus
  progress?: ModelProgress
  error?: string
  onDownload: () => void
  onCancel: () => void
  onRemove: () => void
}) {
  const isDownloading = model.isDownloading
  const pct = progress?.percentage ?? 0
  const speed = progress ? formatSpeed(progress.bytesPerSecond) : ""

  return (
    <div className="rounded-xl border border-border bg-card/40 p-4">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <h3 className="truncate text-sm font-semibold">{model.name}</h3>
            {model.recommended ? (
              <span className="rounded-full border border-primary/40 bg-primary/10 px-2 py-0.5 text-[10px] font-medium text-primary">
                Recommended
              </span>
            ) : null}
            {model.isDownloaded ? (
              <span className="inline-flex items-center gap-1 rounded-full border border-emerald-500/40 bg-emerald-500/10 px-2 py-0.5 text-[10px] font-medium text-emerald-500">
                <Check className="size-3" /> Installed
              </span>
            ) : null}
          </div>
          <p className="mt-1 text-xs text-muted-foreground">
            {model.description}
          </p>
          <p className="mt-1 text-[11px] text-muted-foreground/80">
            {formatBytes(model.sizeBytes)}
            {model.languages.length > 0
              ? ` · ${model.languages.join(", ")}`
              : ""}
          </p>
        </div>

        <div className="flex shrink-0 items-center gap-2">
          {isDownloading ? (
            <Button size="sm" variant="outline" onClick={onCancel}>
              <X /> Cancel
            </Button>
          ) : model.isDownloaded ? (
            <Button size="sm" variant="destructive" onClick={onRemove}>
              <Trash2 /> Remove
            </Button>
          ) : (
            <Button size="sm" onClick={onDownload}>
              <Download /> Download
            </Button>
          )}
        </div>
      </div>

      {isDownloading ? (
        <div className="mt-3 space-y-1.5">
          <Progress value={pct} />
          <div className="flex items-center justify-between text-[11px] text-muted-foreground">
            <span className="inline-flex items-center gap-1">
              <Loader2 className="size-3 animate-spin" />
              {pct > 0 ? `${pct.toFixed(0)}%` : "Starting…"}
            </span>
            {speed ? <span>{speed}</span> : null}
          </div>
        </div>
      ) : null}

      {error ? (
        <p className="mt-2 text-[11px] text-destructive">{error}</p>
      ) : null}
    </div>
  )
}
