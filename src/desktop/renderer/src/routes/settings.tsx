import { createFileRoute } from "@tanstack/react-router"

import { AuthSettings } from "@/components/settings/auth-settings"
import { AudioSettings } from "@/components/settings/audio-settings"
import { GeneralSettings } from "@/components/settings/general-settings"
import { OutputSettings } from "@/components/settings/output-settings"
import { PerformanceSettings } from "@/components/settings/performance-settings"
import { ShortcutSettings } from "@/components/settings/shortcut-settings"
import { TranscriptionSettings } from "@/components/settings/transcription-settings"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"

export const Route = createFileRoute("/settings")({
  component: SettingsPage,
})

function SettingsPage() {
  return (
    <>
      <header>
        <h1 className="text-3xl font-semibold tracking-tight">Settings</h1>
        <p className="mt-2 text-sm text-muted-foreground">Configure how Sonar works on this device.</p>
      </header>

      <Tabs defaultValue="general" className="mt-6">
        <TabsList variant="line" className="grid h-auto w-full grid-cols-7">
          <TabsTrigger value="general">General</TabsTrigger>
          <TabsTrigger value="audio">Audio</TabsTrigger>
          <TabsTrigger value="shortcuts">Shortcuts</TabsTrigger>
          <TabsTrigger value="output">Output</TabsTrigger>
          <TabsTrigger value="transcription">Transcription</TabsTrigger>
          <TabsTrigger value="performance">Performance</TabsTrigger>
          <TabsTrigger value="auth">Auth</TabsTrigger>
        </TabsList>
        <TabsContent value="general" className="min-h-64 pt-4">
          <GeneralSettings />
        </TabsContent>
        <TabsContent value="audio" className="min-h-64 pt-4"><AudioSettings /></TabsContent>
        <TabsContent value="shortcuts" className="min-h-64 pt-4"><ShortcutSettings /></TabsContent>
        <TabsContent value="output" className="min-h-64 pt-4"><OutputSettings /></TabsContent>
        <TabsContent value="transcription" className="min-h-64 pt-4"><TranscriptionSettings /></TabsContent>
        <TabsContent value="performance" className="min-h-64 pt-4"><PerformanceSettings /></TabsContent>
        <TabsContent value="auth" className="min-h-64 pt-4">
          <AuthSettings />
        </TabsContent>
      </Tabs>
    </>
  )
}
