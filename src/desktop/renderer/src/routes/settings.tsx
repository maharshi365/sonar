import { createFileRoute } from "@tanstack/react-router"
import { Settings } from "lucide-react"

import { EmptyPage } from "@/components/empty-page"

export const Route = createFileRoute("/settings")({
  component: SettingsPage,
})

function SettingsPage() {
  return (
    <EmptyPage
      title="No settings yet"
      description="Application preferences will appear here."
      icon={<Settings />}
    />
  )
}
