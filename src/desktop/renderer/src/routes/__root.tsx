import { Outlet, createRootRoute } from "@tanstack/react-router"

import { AppShell } from "@/components/app-shell"

export const Route = createRootRoute({
  component: RootLayout,
  notFoundComponent: () => <p className="text-muted-foreground">Page not found.</p>,
})

function RootLayout() {
  return (
    <AppShell>
      <div className="min-h-full w-full px-8 py-10 lg:px-12">
        <Outlet />
      </div>
    </AppShell>
  )
}
