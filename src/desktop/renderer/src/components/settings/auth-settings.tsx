import { useEffect, useState } from "react"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"

import type { Settings, SettingsPatch } from "../../../../shared/settings"
import { Input } from "@/components/ui/input"
import { Skeleton } from "@/components/ui/skeleton"
import { SettingsGroup, SettingsRow } from "./settings-field"

const settingsQueryKey = ["settings"] as const

export function AuthSettings() {
  const settingsQuery = useQuery({
    queryKey: settingsQueryKey,
    queryFn: () => window.sonar.settings.get(),
  })

  if (settingsQuery.isPending) {
    return <Skeleton className="h-28 w-full" />
  }

  if (settingsQuery.isError) {
    return <p className="text-sm text-destructive">Failed to load settings.</p>
  }

  return <AuthSettingsForm settings={settingsQuery.data} />
}

function AuthSettingsForm({ settings }: { settings: Settings }) {
  const queryClient = useQueryClient()

  const mutation = useMutation({
    mutationFn: (patch: SettingsPatch) => window.sonar.settings.set(patch),
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
    <div className="space-y-3">
      <SettingsGroup title="Model downloads">
        <SettingsRow
          label="Hugging Face access token"
          description="Optional. Lifts anonymous download rate limits. Stored locally and only sent to huggingface.co."
        >
          <Input
            id="huggingFaceToken"
            name="huggingFaceToken"
            className="w-72"
            type="password"
            autoComplete="off"
            value={huggingFaceToken}
            placeholder="hf_..."
            onChange={(event) => setHuggingFaceToken(event.target.value)}
          />
        </SettingsRow>
      </SettingsGroup>
      <div className="min-h-4 text-xs">
        {mutation.isPending ? (
          <span className="text-muted-foreground">Saving...</span>
        ) : mutation.isError ? (
          <span className="text-destructive">Couldn't save</span>
        ) : mutation.isSuccess ? (
          <span className="text-muted-foreground">Saved</span>
        ) : null}
      </div>
    </div>
  )
}
