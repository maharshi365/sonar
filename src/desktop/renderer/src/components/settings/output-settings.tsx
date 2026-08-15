import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Skeleton } from "@/components/ui/skeleton"
import { useSettings } from "@/hooks/use-settings"
import { SettingsGroup, SettingsRow, Toggle } from "./settings-field"

export function OutputSettings() {
  const { query, mutation } = useSettings()
  if (query.isPending) return <Skeleton className="h-64 max-w-2xl" />
  if (query.isError) return <p className="text-sm text-destructive">Failed to load output settings.</p>
  const output = query.data.output
  const save = (patch: Partial<typeof output>) => mutation.mutate({ output: patch })
  const methodLabels = { paste: "Paste into app", clipboard: "Copy to clipboard", none: "Do nothing" }
  const submitLabels = { enter: "Enter", ctrl_enter: "Ctrl + Enter", cmd_enter: "Cmd + Enter" }
  return (
    <SettingsGroup title="Text delivery">
      <SettingsRow label="After transcription" description="Paste into the focused app, copy only, or leave the result in Sonar.">
        <Select value={output.method} onValueChange={(value) => save({ method: value as typeof output.method })}>
          <SelectTrigger className="w-48"><SelectValue>{methodLabels[output.method]}</SelectValue></SelectTrigger>
          <SelectContent align="end" alignItemWithTrigger={false}>
            <SelectItem value="paste">Paste into app</SelectItem>
            <SelectItem value="clipboard">Copy to clipboard</SelectItem>
            <SelectItem value="none">Do nothing</SelectItem>
          </SelectContent>
        </Select>
      </SettingsRow>
      <SettingsRow label="Trailing space" description="Append one space to delivered text.">
        <Toggle label="Append trailing space" checked={output.appendTrailingSpace} onChange={(checked) => save({ appendTrailingSpace: checked })} />
      </SettingsRow>
      <SettingsRow label="Submit after paste" description="Send a submit shortcut after text is pasted into the focused app.">
        <Toggle label="Submit after paste" checked={output.autoSubmit} disabled={output.method !== "paste"} onChange={(checked) => save({ autoSubmit: checked })} />
      </SettingsRow>
      {output.autoSubmit && output.method === "paste" ? (
        <SettingsRow label="Submit shortcut" description="Choose the key chord sent after paste.">
          <Select value={output.autoSubmitKey} onValueChange={(value) => save({ autoSubmitKey: value as typeof output.autoSubmitKey })}>
            <SelectTrigger className="w-40"><SelectValue>{submitLabels[output.autoSubmitKey]}</SelectValue></SelectTrigger>
            <SelectContent align="end" alignItemWithTrigger={false}>
              <SelectItem value="enter">Enter</SelectItem>
              <SelectItem value="ctrl_enter">Ctrl + Enter</SelectItem>
              {window.sonar.platform === "darwin" ? <SelectItem value="cmd_enter">Cmd + Enter</SelectItem> : null}
            </SelectContent>
          </Select>
        </SettingsRow>
      ) : null}
    </SettingsGroup>
  )
}
