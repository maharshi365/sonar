import { useQuery } from "@tanstack/react-query"

import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Skeleton } from "@/components/ui/skeleton"
import { useSettings } from "@/hooks/use-settings"
import { SettingsGroup, SettingsRow } from "./settings-field"

export function PerformanceSettings() {
  const { query, mutation } = useSettings()
  const devices = useQuery({ queryKey: ["compute-devices"], queryFn: () => window.sonar.devices.compute() })
  if (query.isPending || devices.isPending) return <Skeleton className="h-44 max-w-2xl" />
  if (query.isError || devices.isError) return <p className="text-sm text-destructive">Failed to load compute settings.</p>
  const inference = query.data.inference
  const save = (patch: Partial<typeof inference>) => mutation.mutate({ inference: patch })
  const acceleratorLabels = { auto: "Automatic", cpu: "CPU", gpu: "GPU" }
  const selectedDevice = devices.data.find((device) => device.id === inference.gpuDeviceId)
  return (
    <SettingsGroup title="Inference">
      <SettingsRow label="Accelerator" description="Auto chooses the fastest available backend; CPU is the compatibility option.">
        <Select value={inference.accelerator} onValueChange={(value) => save({ accelerator: value as typeof inference.accelerator })}>
          <SelectTrigger className="w-40"><SelectValue>{acceleratorLabels[inference.accelerator]}</SelectValue></SelectTrigger>
          <SelectContent align="end" alignItemWithTrigger={false}>
            <SelectItem value="auto">Automatic</SelectItem>
            <SelectItem value="cpu">CPU</SelectItem>
            <SelectItem value="gpu">GPU</SelectItem>
          </SelectContent>
        </Select>
      </SettingsRow>
      <SettingsRow label="Compute device" description="Choose a registered GPU, or let the backend select one.">
        <Select value={inference.gpuDeviceId || "auto"} disabled={inference.accelerator === "cpu"} onValueChange={(value) => save({ gpuDeviceId: value === "auto" ? "" : String(value) })}>
          <SelectTrigger className="w-64"><SelectValue>{selectedDevice ? `${selectedDevice.name} (${selectedDevice.kind})` : "Automatic"}</SelectValue></SelectTrigger>
          <SelectContent align="end" alignItemWithTrigger={false}>
            <SelectItem value="auto">Automatic</SelectItem>
            {devices.data.filter((device) => !["cpu", "accel"].includes(device.kind.toLowerCase())).map((device) => (
              <SelectItem key={device.id} value={device.id}>{device.name} ({device.kind})</SelectItem>
            ))}
          </SelectContent>
        </Select>
      </SettingsRow>
    </SettingsGroup>
  )
}
