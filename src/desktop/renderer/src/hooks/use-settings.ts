import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"

import type { Settings, SettingsPatch } from "../../../shared/settings"

export const settingsQueryKey = ["settings"] as const

export function useSettings() {
  const queryClient = useQueryClient()
  const query = useQuery({
    queryKey: settingsQueryKey,
    queryFn: () => window.sonar.settings.get(),
  })
  const mutation = useMutation({
    mutationFn: (patch: SettingsPatch) => window.sonar.settings.set(patch),
    onMutate: (patch) => {
      queryClient.setQueryData<Settings>(settingsQueryKey, (current) =>
        current
          ? {
              ...current,
              general: { ...current.general, ...patch.general },
              shortcuts: { ...current.shortcuts, ...patch.shortcuts },
              audio: { ...current.audio, ...patch.audio },
              output: { ...current.output, ...patch.output },
              transcription: {
                ...current.transcription,
                ...patch.transcription,
              },
              inference: { ...current.inference, ...patch.inference },
              auth: { ...current.auth, ...patch.auth },
            }
          : current
      )
    },
    onSuccess: (settings) => queryClient.setQueryData(settingsQueryKey, settings),
    onError: () => void queryClient.invalidateQueries({ queryKey: settingsQueryKey }),
  })
  return { query, mutation }
}
