import {
  AudioWaveform,
  Clock3,
  Download,
  HardDrive,
  Languages,
  Loader2,
  Radio,
  Star,
  Trash2,
  X,
} from "lucide-react"

import type { ModelStatus } from "../../../../shared/models"
import type { ModelProgress } from "@/hooks/use-models"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
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

function formatLanguages(languages: string[]): string {
  return languages
    .map((language) => {
      if (language === "en") return "English"
      if (language === "multilingual") return "Multilingual"
      return language.toUpperCase()
    })
    .join(", ")
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
    <Card size="sm" className="bg-card/40">
      <CardHeader>
        <div className="flex min-w-0 gap-2.5">
          <div className="grid size-8 shrink-0 place-items-center rounded-md border border-border bg-secondary text-primary">
            <AudioWaveform className="size-4" />
          </div>

          <div className="min-w-0">
            <CardTitle className="truncate">{model.name}</CardTitle>
            <CardDescription className="mt-0.5 line-clamp-1 text-xs">
              {model.description}
            </CardDescription>
          </div>
        </div>

        <CardAction>
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
        </CardAction>
      </CardHeader>

      {isDownloading || error ? (
        <CardContent className="space-y-2">
          {isDownloading ? (
            <div className="space-y-1.5">
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
          {error ? <p className="text-[11px] text-destructive">{error}</p> : null}
        </CardContent>
      ) : null}

      <CardFooter className="justify-between gap-2 py-2">
        <div className="flex flex-1 flex-wrap items-center gap-2 text-[11px]">
          <span
            className={`inline-flex items-center gap-1.5 rounded-md border px-2 py-1 font-medium ${
              model.supportsStreaming
                ? "border-cyan-500/40 bg-cyan-500/10 text-cyan-600 dark:text-cyan-400"
                : "border-border bg-secondary text-muted-foreground"
            }`}
          >
            {model.supportsStreaming ? (
              <Radio className="size-3.5" />
            ) : (
              <Clock3 className="size-3.5" />
            )}
            {model.supportsStreaming ? "Live streaming" : "Transcribes on stop"}
          </span>
          <span className="inline-flex items-center gap-1.5 px-1 text-muted-foreground">
            <HardDrive className="size-3.5" />
            {formatBytes(model.sizeBytes)}
          </span>
          {model.languages.length > 0 ? (
            <span className="inline-flex items-center gap-1.5 px-1 text-muted-foreground">
              <Languages className="size-3.5" />
              {formatLanguages(model.languages)}
            </span>
          ) : null}
        </div>
        {model.recommended ? (
          <span
            className="shrink-0 pb-1 text-primary"
            title="Recommended model"
            aria-label="Recommended model"
          >
            <Star className="size-4 fill-primary/20" />
          </span>
        ) : null}
      </CardFooter>
    </Card>
  )
}
