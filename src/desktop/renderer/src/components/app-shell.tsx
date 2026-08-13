import type { ReactNode } from "react"
import { AudioLines, History, Settings } from "lucide-react"

import { Button } from "@/components/ui/button"

export function AppShell({ children }: { children: ReactNode }) {
  return (
    <div className="flex min-h-screen bg-background text-foreground">
      <aside className="flex w-56 shrink-0 flex-col border-r border-border bg-card/40 pt-11">
        <div className="flex h-16 items-center gap-3 px-5">
          <div className="grid size-8 place-items-center rounded-lg bg-primary text-primary-foreground shadow-[0_0_24px_-6px_var(--primary)]">
            <AudioLines className="size-4" />
          </div>
          <span className="text-sm font-semibold tracking-wide">SONAR</span>
        </div>

        <nav className="space-y-1 px-3 py-2" aria-label="Primary navigation">
          <Button className="w-full justify-start" variant="secondary">
            <AudioLines />
            Transcribe
          </Button>
          <Button className="w-full justify-start text-muted-foreground" variant="ghost" disabled>
            <History />
            History
          </Button>
        </nav>

        <div className="mt-auto p-3">
          <Button className="w-full justify-start text-muted-foreground" variant="ghost" disabled>
            <Settings />
            Settings
          </Button>
        </div>
      </aside>

      <div className="min-w-0 flex-1 pt-11">
        <div className="fixed inset-x-0 top-0 z-50 h-11 border-b border-border/70 bg-background/90 [-webkit-app-region:drag]" />
        <main className="h-[calc(100vh-2.75rem)] overflow-y-auto">{children}</main>
      </div>
    </div>
  )
}
