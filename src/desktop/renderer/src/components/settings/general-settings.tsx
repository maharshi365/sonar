import { useForm } from "@tanstack/react-form"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { Link } from "@tanstack/react-router"

import type { ModelStatus } from "../../../../shared/models"
import type { Settings } from "../../../../shared/settings"
import { Button } from "@/components/ui/button"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Skeleton } from "@/components/ui/skeleton"

const settingsQueryKey = ["settings"] as const
const modelsQueryKey = ["models"] as const

export function GeneralSettings() {
  const settingsQuery = useQuery({
    queryKey: settingsQueryKey,
    queryFn: () => window.sonar.settings.get(),
  })

  const modelsQuery = useQuery({
    queryKey: modelsQueryKey,
    queryFn: () => window.sonar.models.list(),
  })

  if (settingsQuery.isPending || modelsQuery.isPending) {
    return (
      <div className="max-w-xl space-y-6">
        <div className="space-y-2">
          <Skeleton className="h-4 w-40" />
          <Skeleton className="h-8 w-full" />
          <Skeleton className="h-3 w-64" />
        </div>
        <Skeleton className="h-8 w-32" />
      </div>
    )
  }

  if (settingsQuery.isError || modelsQuery.isError) {
    return (
      <p className="text-sm text-destructive">Failed to load settings.</p>
    )
  }

  // Mount the form only once settings are loaded so the form's default values
  // reflect the persisted state.
  return (
    <GeneralSettingsForm
      settings={settingsQuery.data}
      models={modelsQuery.data}
    />
  )
}

function GeneralSettingsForm({
  settings,
  models,
}: {
  settings: Settings
  models: ModelStatus[]
}) {
  const queryClient = useQueryClient()

  const downloadedModels = models.filter((model) => model.isDownloaded)

  const mutation = useMutation({
    mutationFn: (patch: Partial<Settings>) => window.sonar.settings.set(patch),
    onSuccess: (next) => {
      queryClient.setQueryData(settingsQueryKey, next)
    },
  })

  // If the persisted model is no longer downloaded, don't preselect it.
  const initialModel = downloadedModels.some(
    (model) => model.id === settings.general.ttsModel
  )
    ? settings.general.ttsModel
    : ""

  const form = useForm({
    defaultValues: {
      ttsModel: initialModel,
    },
    onSubmit: async ({ value }) => {
      await mutation.mutateAsync({ general: { ttsModel: value.ttsModel } })
    },
  })

  return (
    <form
      className="max-w-xl space-y-6"
      onSubmit={(event) => {
        event.preventDefault()
        void form.handleSubmit()
      }}
    >
      <form.Field name="ttsModel">
        {(field) => (
          <div className="space-y-2">
            <label htmlFor={field.name} className="text-sm font-medium">
              Default speech model
            </label>
            {downloadedModels.length > 0 ? (
              <Select
                value={field.state.value}
                onValueChange={(value) => field.handleChange(value as string)}
                disabled={mutation.isPending}
              >
                <SelectTrigger
                  id={field.name}
                  className="w-full"
                  onBlur={field.handleBlur}
                >
                  <SelectValue placeholder="Select a model">
                    {(value: string) =>
                      downloadedModels.find((model) => model.id === value)
                        ?.name ?? "Select a model"
                    }
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  {downloadedModels.map((model) => (
                    <SelectItem key={model.id} value={model.id}>
                      {model.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            ) : (
              <p className="rounded-lg border border-border bg-card/40 px-3 py-2 text-xs text-muted-foreground">
                No models downloaded yet.{" "}
                <Link to="/models" className="text-primary hover:underline">
                  Download a model
                </Link>{" "}
                to select it here.
              </p>
            )}
            <p className="text-xs text-muted-foreground">
              The model used for transcription by default. Only downloaded
              models can be selected.
            </p>
          </div>
        )}
      </form.Field>

      <div className="flex items-center gap-3">
        <form.Subscribe selector={(state) => state.isSubmitting}>
          {(isSubmitting) => (
            <Button
              type="submit"
              disabled={
                isSubmitting ||
                mutation.isPending ||
                downloadedModels.length === 0
              }
            >
              {mutation.isPending ? "Saving…" : "Save changes"}
            </Button>
          )}
        </form.Subscribe>
        {mutation.isSuccess && !mutation.isPending ? (
          <span className="text-xs text-muted-foreground">Saved</span>
        ) : null}
      </div>
    </form>
  )
}
