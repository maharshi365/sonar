import { useEffect, useState } from "react"

import type { TranscriptionSettings as TranscriptionValues } from "../../../../shared/settings"
import { Input, NumberInput } from "@/components/ui/input"
import { Slider } from "@/components/ui/slider"
import { Skeleton } from "@/components/ui/skeleton"
import { useSettings } from "@/hooks/use-settings"
import { SettingsGroup, SettingsRow, Toggle } from "./settings-field"

export function TranscriptionSettings() {
  const { query, mutation } = useSettings()
  if (query.isPending) return <Skeleton className="h-96 max-w-2xl" />
  if (query.isError) return <p className="text-sm text-destructive">Failed to load transcription settings.</p>
  return <TranscriptionForm values={query.data.transcription} save={(patch) => mutation.mutate({ transcription: patch })} />
}

function TranscriptionForm({ values, save }: { values: TranscriptionValues; save: (patch: Partial<TranscriptionValues>) => void }) {
  const [customWords, setCustomWords] = useState(values.customWords.join(", "))
  const [fillerWords, setFillerWords] = useState(values.customFillerWords.join(", "))
  const [threshold, setThreshold] = useState(values.wordCorrectionThreshold)
  useEffect(() => setCustomWords(values.customWords.join(", ")), [values.customWords])
  useEffect(() => setFillerWords(values.customFillerWords.join(", ")), [values.customFillerWords])
  useEffect(() => setThreshold(values.wordCorrectionThreshold), [values.wordCorrectionThreshold])
  const words = (text: string) => text.split(",").map((word) => word.trim()).filter(Boolean)
  return (
    <SettingsGroup title="Recognition">
      <SettingsRow label="Custom words" description="Names and technical terms to prompt and correct. Separate entries with commas.">
        <Input className="w-72" value={customWords} placeholder="Sonar, Acme Corp" onChange={(event) => setCustomWords(event.target.value)} onBlur={() => save({ customWords: words(customWords) })} />
      </SettingsRow>
      <SettingsRow label="Remove filler words" description="Remove conservative non-lexical fillers such as uh, uhm, and hmm.">
        <Toggle label="Remove filler words" checked={values.fillerWordRemoval} onChange={(checked) => save({ fillerWordRemoval: checked })} />
      </SettingsRow>
      <SettingsRow label="Custom filler words" description="Optional explicit filler list. Separate entries with commas.">
        <Input className="w-72" value={fillerWords} placeholder="um, like" onChange={(event) => setFillerWords(event.target.value)} onBlur={() => save({ customFillerWords: words(fillerWords) })} />
      </SettingsRow>
      <SettingsRow label="Trailing audio buffer" description="Continue recording briefly after stop to avoid clipping the final word.">
        <div className="flex items-center gap-2"><NumberInput className="w-24" min={0} max={5000} step={50} defaultValue={values.extraRecordingBufferMs} onBlur={(event) => save({ extraRecordingBufferMs: Number(event.target.value) })} /><span className="text-xs text-muted-foreground">ms</span></div>
      </SettingsRow>
      <SettingsRow label="Word correction threshold" description="Higher values apply more aggressive fuzzy corrections to custom words.">
        <div className="flex w-64 items-center gap-3">
          <Slider
            aria-label="Word correction threshold"
            value={threshold}
            min={0}
            max={1}
            step={0.01}
            onValueChange={(value) => {
              if (typeof value === "number") setThreshold(value)
            }}
            onValueCommitted={(value) => {
              if (typeof value === "number") save({ wordCorrectionThreshold: value })
            }}
          />
          <span className="w-8 text-right text-xs tabular-nums text-muted-foreground">
            {threshold.toFixed(2)}
          </span>
        </div>
      </SettingsRow>
    </SettingsGroup>
  )
}
