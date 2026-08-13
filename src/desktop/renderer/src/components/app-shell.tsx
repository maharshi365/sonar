import type { ReactNode } from "react"
import { Link } from "@tanstack/react-router"
import { AudioLines, Box, History, Settings } from "lucide-react"

import { buttonVariants } from "@/components/ui/button"
import { cn } from "@/lib/utils"

const navLinkClass = cn(
  buttonVariants({ variant: "ghost" }),
  "w-full justify-start text-muted-foreground",
)

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
          <Link
            to="/"
            activeOptions={{ exact: true }}
            activeProps={{ className: "bg-secondary text-secondary-foreground" }}
            className={navLinkClass}
          >
            <AudioLines />
            Transcribe
          </Link>
          <Link
            to="/history"
            activeProps={{ className: "bg-secondary text-secondary-foreground" }}
            className={navLinkClass}
          >
            <History />
            History
          </Link>
          <Link
            to="/models"
            activeProps={{ className: "bg-secondary text-secondary-foreground" }}
            className={navLinkClass}
          >
            <Box />
            Models
          </Link>
        </nav>

        <div className="mt-auto p-3">
          <Link
            to="/settings"
            activeProps={{ className: "bg-secondary text-secondary-foreground" }}
            className={navLinkClass}
          >
            <Settings />
            Settings
          </Link>
        </div>
      </aside>

      <div className="min-w-0 flex-1 pt-11">
        <div className="fixed inset-x-0 top-0 z-50 h-11 border-b border-border/70 bg-background/90 [-webkit-app-region:drag]" />
        <main className="h-[calc(100vh-2.75rem)] overflow-y-auto">{children}</main>
      </div>
    </div>
  )
}
