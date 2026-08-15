import { useEffect, useState } from "react"
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { Link } from "@tanstack/react-router"

import type { ModelStatus } from "../../../../shared/models"
import type { Settings, SettingsPatch } from "../../../../shared/settings"
import type { UpdateStatus } from "../../../../shared/updates"
import { Button } from "@/components/ui/button"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Skeleton } from "@/components/ui/skeleton"
import { NumberInput } from "@/components/ui/input"
import { SettingsGroup, SettingsRow } from "./settings-field"

const settingsQueryKey = ["settings"] as const
const modelsQueryKey = ["models"] as const
const updateQueryKey = ["update-status"] as const

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
    mutationFn: (patch: SettingsPatch) => window.sonar.settings.set(patch),
    onMutate: (patch) => {
      queryClient.setQueryData<Settings>(settingsQueryKey, (current) =>
        current && patch.general
          ? { ...current, general: { ...current.general, ...patch.general } }
          : current
      )
    },
    onSuccess: (next) => {
      queryClient.setQueryData(settingsQueryKey, next)
    },
    onError: () => void queryClient.invalidateQueries({ queryKey: settingsQueryKey }),
  })

  // If the persisted model is no longer downloaded, don't preselect it.
  const initialModel = downloadedModels.some(
    (model) => model.id === settings.general.ttsModel
  )
    ? settings.general.ttsModel
    : ""

  const [ttsModel, setTtsModel] = useState(initialModel)

  const saveGeneral = (patch: Partial<typeof settings.general>) =>
    mutation.mutate({ general: patch })
  const unloadLabels = {
    immediately: "Immediately",
    "2m": "After 2 minutes",
    "5m": "After 5 minutes",
    "10m": "After 10 minutes",
    "15m": "After 15 minutes",
    "1h": "After 1 hour",
    never: "Never",
  }

  return (
    <div className="space-y-6">
      <SettingsGroup title="Models and history">
      <SettingsRow label="Default speech model" description="The downloaded model used for transcription.">
      {downloadedModels.length > 0 ? (
        <div className="flex items-center gap-3">
          <Select
            value={ttsModel}
            onValueChange={(value) => {
              const nextModel = value as string
              setTtsModel(nextModel)
              saveGeneral({ ttsModel: nextModel })
            }}
            disabled={mutation.isPending}
          >
            <SelectTrigger id="ttsModel" className="w-72 max-w-full">
              <SelectValue placeholder="Select a model">
                {(value: string) =>
                  downloadedModels.find((model) => model.id === value)?.name ??
                  "Select a model"
                }
              </SelectValue>
            </SelectTrigger>
            <SelectContent align="start" alignItemWithTrigger={false}>
              {downloadedModels.map((model) => (
                <SelectItem key={model.id} value={model.id}>
                  {model.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {mutation.isPending ? (
            <span className="text-xs text-muted-foreground">Saving…</span>
          ) : mutation.isError ? (
            <span className="text-xs text-destructive">Couldn’t save</span>
          ) : mutation.isSuccess ? (
            <span className="text-xs text-muted-foreground">Saved</span>
          ) : null}
        </div>
      ) : (
        <p className="rounded-lg border border-border bg-card/40 px-3 py-2 text-xs text-muted-foreground">
          No models downloaded yet.{" "}
          <Link to="/models" className="text-primary hover:underline">
            Download a model
          </Link>{" "}
          to select it here.
        </p>
      )}
      </SettingsRow>
      <SettingsRow label="Unload model" description="Release model memory after Sonar has been idle.">
        <Select value={settings.general.modelUnloadTimeout} onValueChange={(value) => saveGeneral({ modelUnloadTimeout: value as typeof settings.general.modelUnloadTimeout })}>
          <SelectTrigger className="w-40">
            <SelectValue>{(value: keyof typeof unloadLabels) => unloadLabels[value]}</SelectValue>
          </SelectTrigger>
          <SelectContent align="end" alignItemWithTrigger={false}>
            <SelectItem value="immediately">Immediately</SelectItem>
            <SelectItem value="2m">After 2 minutes</SelectItem>
            <SelectItem value="5m">After 5 minutes</SelectItem>
            <SelectItem value="10m">After 10 minutes</SelectItem>
            <SelectItem value="15m">After 15 minutes</SelectItem>
            <SelectItem value="1h">After 1 hour</SelectItem>
            <SelectItem value="never">Never</SelectItem>
          </SelectContent>
        </Select>
      </SettingsRow>
      <SettingsRow label="History limit" description="Maximum saved transcripts. Set to 0 to disable and clear history.">
        <NumberInput className="w-24" min={0} max={10000} defaultValue={settings.general.historyLimit} onBlur={(event) => saveGeneral({ historyLimit: Number(event.target.value) })} />
      </SettingsRow>
      </SettingsGroup>
      <UpdateSettings />
    </div>
  )
}

function UpdateSettings() {
  const queryClient = useQueryClient()
  const statusQuery = useQuery({
    queryKey: updateQueryKey,
    queryFn: () => window.sonar.updates.getStatus(),
  })
  const checkMutation = useMutation({
    mutationFn: () => window.sonar.updates.check(),
    onSuccess: (status) => queryClient.setQueryData(updateQueryKey, status),
  })
  const installMutation = useMutation({
    mutationFn: () => window.sonar.updates.install(),
  })

  useEffect(
    () =>
      window.sonar.updates.onStatus((status) => {
        queryClient.setQueryData(updateQueryKey, status)
      }),
    [queryClient]
  )

  const status = statusQuery.data
  const description = updateDescription(status)
  const disabled =
    !status ||
    status.phase === "unsupported" ||
    status.phase === "checking" ||
    status.phase === "available" ||
    status.phase === "downloading" ||
    checkMutation.isPending ||
    installMutation.isPending

  return (
    <SettingsGroup title="Updates">
      <SettingsRow label="Sonar updates" description={description}>
        <Button
          variant={status?.phase === "downloaded" ? "default" : "outline"}
          disabled={disabled}
          onClick={() => {
            if (status?.phase === "downloaded") installMutation.mutate()
            else checkMutation.mutate()
          }}
        >
          {updateButtonLabel(status)}
        </Button>
      </SettingsRow>
    </SettingsGroup>
  )
}

function updateDescription(status: UpdateStatus | undefined): string {
  if (!status) return "Loading update status..."
  const version = `Version ${status.currentVersion}.`

  switch (status.phase) {
    case "checking":
      return `${version} Checking for updates...`
    case "up-to-date":
      return `${version} Sonar is up to date.`
    case "available":
      return `${version} Downloading Sonar ${status.version}...`
    case "downloading":
      return `${version} Downloading Sonar ${status.version} (${status.percent ?? 0}%).`
    case "downloaded":
      return `${version} Sonar ${status.version} is ready to install.`
    case "error":
      return `${version} ${status.message ?? "The update check failed."}`
    case "unsupported":
      return `${version} ${status.message}`
    default:
      return `${version} Check GitHub for a newer release.`
  }
}

function updateButtonLabel(status: UpdateStatus | undefined): string {
  if (!status) return "Loading..."
  switch (status.phase) {
    case "checking":
      return "Checking..."
    case "available":
      return "Starting download..."
    case "downloading":
      return `Downloading ${status.percent ?? 0}%`
    case "downloaded":
      return "Install and restart"
    case "unsupported":
      return "Installed builds only"
    default:
      return "Check for updates"
  }
}
