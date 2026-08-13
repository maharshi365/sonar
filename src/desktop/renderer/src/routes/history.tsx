import { createFileRoute } from "@tanstack/react-router"
import { History } from "lucide-react"

import { EmptyPage } from "@/components/empty-page"

export const Route = createFileRoute("/history")({
  component: HistoryPage,
})

function HistoryPage() {
  return (
    <EmptyPage
      title="No transcription history"
      description="Your completed transcriptions will appear here."
      icon={<History />}
    />
  )
}
