import { createFileRoute } from "@tanstack/react-router"
import { Box } from "lucide-react"

import { EmptyPage } from "@/components/empty-page"

export const Route = createFileRoute("/models")({
  component: ModelsPage,
})

function ModelsPage() {
  return (
    <EmptyPage
      title="No models installed"
      description="Installed speech models will appear here."
      icon={<Box />}
    />
  )
}
