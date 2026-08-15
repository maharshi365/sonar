import { useQuery } from "@tanstack/react-query"

import { SettingsGroup, SettingsRow } from "./settings-field"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Skeleton } from "@/components/ui/skeleton"
import { useSettings } from "@/hooks/use-settings"

export function AudioSettings() {
  const { query, mutation } = useSettings()
  const devices = useQuery({
    queryKey: ["audio-input-devices"],
    queryFn: () => window.sonar.devices.inputs(),
  })
  if (query.isPending || devices.isPending) return <Skeleton className="h-24 max-w-2xl" />
  if (query.isError || devices.isError) return <p className="text-sm text-destructive">Failed to load audio devices.</p>

  const value = query.data.audio.inputDeviceId || "default"
  const selectedLabel = value === "default"
    ? "System default"
    : devices.data.find((device) => device.id === value)?.name ?? value
  return (
    <SettingsGroup title="Input">
      <SettingsRow label="Microphone" description="The input Sonar records. System default follows OS device changes.">
        <Select
          value={value}
          disabled={mutation.isPending}
          onValueChange={(next) => mutation.mutate({ audio: { inputDeviceId: next === "default" ? "" : String(next) } })}
        >
          <SelectTrigger className="w-64"><SelectValue>{selectedLabel}</SelectValue></SelectTrigger>
          <SelectContent align="end" alignItemWithTrigger={false}>
            <SelectItem value="default">System default</SelectItem>
            {devices.data.map((device) => (
              <SelectItem key={device.id} value={device.id}>{device.name}{device.isDefault ? " (default)" : ""}</SelectItem>
            ))}
          </SelectContent>
        </Select>
      </SettingsRow>
    </SettingsGroup>
  )
}
