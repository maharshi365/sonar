import { useForm } from "@tanstack/react-form"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"

import type { Settings } from "../../../../shared/settings"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Skeleton } from "@/components/ui/skeleton"

const settingsQueryKey = ["settings"] as const

export function AuthSettings() {
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
    return <p className="text-sm text-destructive">Failed to load settings.</p>
  }

  return <AuthSettingsForm settings={settingsQuery.data} />
}

function AuthSettingsForm({ settings }: { settings: Settings }) {
  const queryClient = useQueryClient()

  const mutation = useMutation({
    mutationFn: (patch: Partial<Settings>) => window.sonar.settings.set(patch),
    onSuccess: (next) => {
      queryClient.setQueryData(settingsQueryKey, next)
    },
  })

  const form = useForm({
    defaultValues: {
      huggingFaceToken: settings.auth.huggingFaceToken,
    },
    onSubmit: async ({ value }) => {
      await mutation.mutateAsync({
        auth: { huggingFaceToken: value.huggingFaceToken.trim() },
      })
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
      <form.Field name="huggingFaceToken">
        {(field) => (
          <div className="space-y-2">
            <label htmlFor={field.name} className="text-sm font-medium">
              Hugging Face access token
            </label>
            <Input
              id={field.name}
              name={field.name}
              type="password"
              autoComplete="off"
              value={field.state.value}
              disabled={mutation.isPending}
              placeholder="hf_…"
              onBlur={field.handleBlur}
              onChange={(event) => field.handleChange(event.target.value)}
            />
            <p className="text-xs text-muted-foreground">
              Optional. All models are public, but a token lifts anonymous rate
              limits and can speed up downloads. It is stored locally and only
              ever sent to huggingface.co.
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
