import { createFileRoute } from "@tanstack/react-router"

import { AuthSettings } from "@/components/settings/auth-settings"
import { GeneralSettings } from "@/components/settings/general-settings"
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

      <Tabs defaultValue="general" className="mt-8">
        <TabsList variant="line" className="w-full justify-start border-b border-border">
          <TabsTrigger value="general">General</TabsTrigger>
          <TabsTrigger value="auth">Auth</TabsTrigger>
        </TabsList>
        <TabsContent value="general" className="min-h-64 pt-6">
          <GeneralSettings />
        </TabsContent>
        <TabsContent value="auth" className="min-h-64 pt-6">
          <AuthSettings />
        </TabsContent>
      </Tabs>
    </>
  )
}
