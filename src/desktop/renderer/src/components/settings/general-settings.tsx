import { useForm } from "@tanstack/react-form"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"

import type { Settings } from "../../../../shared/settings"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Skeleton } from "@/components/ui/skeleton"

const settingsQueryKey = ["settings"] as const

export function GeneralSettings() {
  const settingsQuery = useQuery({
    queryKey: settingsQueryKey,
    queryFn: () => window.sonar.settings.get(),
  })

  if (settingsQuery.isPending) {
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

  if (settingsQuery.isError) {
    return (
      <p className="text-sm text-destructive">Failed to load settings.</p>
    )
  }

  // Mount the form only once settings are loaded so the form's default values
  // reflect the persisted state.
  return <GeneralSettingsForm settings={settingsQuery.data} />
}

function GeneralSettingsForm({ settings }: { settings: Settings }) {
  const queryClient = useQueryClient()

  const mutation = useMutation({
    mutationFn: (patch: Partial<Settings>) => window.sonar.settings.set(patch),
    onSuccess: (next) => {
      queryClient.setQueryData(settingsQueryKey, next)
    },
  })

  const form = useForm({
    defaultValues: {
      ttsModel: settings.general.ttsModel,
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
            <Input
              id={field.name}
              name={field.name}
              value={field.state.value}
              disabled={mutation.isPending}
              placeholder="e.g. whisper-base"
              onBlur={field.handleBlur}
              onChange={(event) => field.handleChange(event.target.value)}
            />
            <p className="text-xs text-muted-foreground">
              Identifier of the model used for transcription by default.
            </p>
          </div>
        )}
      </form.Field>

      <div className="flex items-center gap-3">
        <form.Subscribe selector={(state) => state.isSubmitting}>
          {(isSubmitting) => (
            <Button type="submit" disabled={isSubmitting || mutation.isPending}>
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
