import { useEffect, useState } from "react"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"

import type { Settings } from "../../../../shared/settings"
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

  const [huggingFaceToken, setHuggingFaceToken] = useState(
    settings.auth.huggingFaceToken
  )

  useEffect(() => {
    const nextToken = huggingFaceToken.trim()
    if (nextToken === settings.auth.huggingFaceToken) return

    const timeout = window.setTimeout(() => {
      mutation.mutate({ auth: { huggingFaceToken: nextToken } })
    }, 500)

    return () => window.clearTimeout(timeout)
  }, [huggingFaceToken, settings.auth.huggingFaceToken, mutation.mutate])

  return (
    <div className="max-w-xl space-y-2">
      <label htmlFor="huggingFaceToken" className="text-sm font-medium">
        Hugging Face access token
      </label>
      <Input
        id="huggingFaceToken"
        name="huggingFaceToken"
        type="password"
        autoComplete="off"
        value={huggingFaceToken}
        placeholder="hf_…"
        onChange={(event) => setHuggingFaceToken(event.target.value)}
      />
      <p className="text-xs text-muted-foreground">
        Optional. All models are public, but a token lifts anonymous rate limits
        and can speed up downloads. It is stored locally and only ever sent to
        huggingface.co.
      </p>
      <div className="min-h-4 text-xs">
        {mutation.isPending ? (
          <span className="text-muted-foreground">Saving…</span>
        ) : mutation.isError ? (
          <span className="text-destructive">Couldn’t save</span>
        ) : mutation.isSuccess ? (
          <span className="text-muted-foreground">Saved</span>
        ) : null}
      </div>
    </div>
  )
}
