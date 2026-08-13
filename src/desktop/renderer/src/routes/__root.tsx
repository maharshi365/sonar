import { Outlet, createRootRoute } from "@tanstack/react-router"

import { AppShell } from "@/components/app-shell"

export const Route = createRootRoute({
  component: () => (
    <AppShell>
      <Outlet />
    </AppShell>
  ),
  notFoundComponent: () => <div className="p-8 text-muted-foreground">Page not found.</div>,
})
