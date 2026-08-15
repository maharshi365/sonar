import { useState } from "react"
import { useHotkeyRecorder } from "@tanstack/react-hotkeys"

import type { ShortcutSettings as ShortcutValues } from "../../../../shared/settings"
import { Button } from "@/components/ui/button"
import { Kbd, KbdGroup } from "@/components/ui/kbd"
import { Skeleton } from "@/components/ui/skeleton"
import { useSettings } from "@/hooks/use-settings"
import { SettingsGroup, SettingsRow } from "./settings-field"

export function ShortcutSettings() {
  const { query, mutation } = useSettings()
  if (query.isPending) return <Skeleton className="h-44 max-w-2xl" />
  if (query.isError) return <p className="text-sm text-destructive">Failed to load shortcuts.</p>
  return <ShortcutForm values={query.data.shortcuts} save={(shortcuts) => mutation.mutate({ shortcuts })} error={mutation.error} />
}

function ShortcutForm({ values, save, error }: { values: ShortcutValues; save: (values: Partial<ShortcutValues>) => void; error: Error | null }) {
  const [editing, setEditing] = useState<keyof ShortcutValues | null>(null)
  return (
    <div className="space-y-3">
      <SettingsGroup title="Global shortcuts">
        <SettingsRow label="Start or stop dictation" description="Click the shortcut, then press a new key combination.">
          <ShortcutField name="transcribe" value={values.transcribe} editing={editing} setEditing={setEditing} save={save} />
        </SettingsRow>
        <SettingsRow label="Cancel dictation" description="Discards the active recording without inserting text.">
          <ShortcutField name="cancel" value={values.cancel} editing={editing} setEditing={setEditing} save={save} />
        </SettingsRow>
      </SettingsGroup>
      {error ? <p className="text-xs text-destructive">{error.message}</p> : null}
    </div>
  )
}

function ShortcutField({
  name,
  value,
  editing,
  setEditing,
  save,
}: {
  name: keyof ShortcutValues
  value: string
  editing: keyof ShortcutValues | null
  setEditing: (name: keyof ShortcutValues | null) => void
  save: (values: Partial<ShortcutValues>) => void
}) {
  const recorder = useHotkeyRecorder({
    ignoreInputs: false,
    onRecord: (hotkey) => {
      const accelerator = hotkey.replace(/^Mod\+/, "CommandOrControl+")
      save({ [name]: accelerator })
      setEditing(null)
    },
    onCancel: () => setEditing(null),
  })
  const isEditing = editing === name && recorder.isRecording

  return (
    <Button
      type="button"
      variant="outline"
      className="min-w-52 justify-center"
      disabled={editing !== null && editing !== name}
      onClick={() => {
        if (isEditing) {
          recorder.cancelRecording()
        } else {
          setEditing(name)
          recorder.startRecording()
        }
      }}
    >
      {isEditing ? (
        <span className="text-muted-foreground">Press shortcut...</span>
      ) : (
        <ShortcutKeys value={value} />
      )}
    </Button>
  )
}

export function ShortcutKeys({ value }: { value: string }) {
  const primaryModifier = window.sonar.platform === "darwin" ? "Cmd" : "Ctrl"
  const labels: Record<string, string> = {
    CommandOrControl: primaryModifier,
    Mod: primaryModifier,
    Control: "Ctrl",
    Command: "Cmd",
    Meta: window.sonar.platform === "darwin" ? "Cmd" : "Win",
    Escape: "Esc",
  }

  return (
    <KbdGroup>
      {value.split("+").map((key) => <Kbd key={key}>{labels[key] ?? key}</Kbd>)}
    </KbdGroup>
  )
}
