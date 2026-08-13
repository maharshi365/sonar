import { createFileRoute } from "@tanstack/react-router"

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"

export const Route = createFileRoute("/settings")({
  component: SettingsPage,
})

function SettingsPage() {
  return (
    <div className="mx-auto min-h-full max-w-4xl px-8 py-10 lg:px-12">
      <header>
        <h1 className="text-3xl font-semibold tracking-tight">Settings</h1>
        <p className="mt-2 text-sm text-muted-foreground">Configure how Sonar works on this device.</p>
      </header>

      <Tabs defaultValue="general" className="mt-8">
        <TabsList variant="line" className="w-full justify-start border-b border-border">
          <TabsTrigger value="general">General</TabsTrigger>
          <TabsTrigger value="models">Models</TabsTrigger>
        </TabsList>
        <TabsContent value="general" className="min-h-64 pt-6" />
        <TabsContent value="models" className="min-h-64 pt-6" />
      </Tabs>
    </div>
  )
}
