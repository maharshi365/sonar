import { createFileRoute } from "@tanstack/react-router"
import { Box } from "lucide-react"

import { ModelCard } from "@/components/models/model-card"
import { Skeleton } from "@/components/ui/skeleton"
import { useModels } from "@/hooks/use-models"

export const Route = createFileRoute("/models")({
  component: ModelsPage,
})

function ModelsPage() {
  const {
    models,
    isLoading,
    error,
    progress,
    errors,
    download,
    cancel,
    remove,
  } = useModels()

  const installed = models.filter((m) => m.isDownloaded || m.isDownloading)
  const available = models.filter((m) => !m.isDownloaded && !m.isDownloading)

  return (
    <>
      <header>
        <h1 className="text-3xl font-semibold tracking-tight">Models</h1>
        <p className="mt-2 text-sm text-muted-foreground">
          Download speech models to use them for transcription. Models are
          stored locally and can be removed at any time.
        </p>
      </header>

      {error ? (
        <p className="mt-6 text-sm text-destructive">
          Failed to load models: {error}
        </p>
      ) : null}

      {isLoading ? (
        <div className="mt-8 space-y-3">
          <Skeleton className="h-24 w-full rounded-xl" />
          <Skeleton className="h-24 w-full rounded-xl" />
          <Skeleton className="h-24 w-full rounded-xl" />
        </div>
      ) : (
        <div className="mt-8 space-y-8">
          {installed.length > 0 ? (
            <section className="space-y-3">
              <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Your models
              </h2>
              {installed.map((model) => (
                <ModelCard
                  key={model.id}
                  model={model}
                  progress={progress[model.id]}
                  error={errors[model.id]}
                  onDownload={() => void download(model.id)}
                  onCancel={() => void cancel(model.id)}
                  onRemove={() => void remove(model.id)}
                />
              ))}
            </section>
          ) : null}

          <section className="space-y-3">
            <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              Available models
            </h2>
            {available.length > 0 ? (
              available.map((model) => (
                <ModelCard
                  key={model.id}
                  model={model}
                  progress={progress[model.id]}
                  error={errors[model.id]}
                  onDownload={() => void download(model.id)}
                  onCancel={() => void cancel(model.id)}
                  onRemove={() => void remove(model.id)}
                />
              ))
            ) : (
              <div className="flex items-center gap-2 rounded-xl border border-border bg-card/20 p-6 text-sm text-muted-foreground">
                <Box className="size-4" />
                All catalog models are installed.
              </div>
            )}
          </section>
        </div>
      )}
    </>
  )
}
