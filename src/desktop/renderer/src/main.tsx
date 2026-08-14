import { StrictMode } from "react"
import { createRoot } from "react-dom/client"
import { QueryClientProvider } from "@tanstack/react-query"
import { RouterProvider } from "@tanstack/react-router"

import { queryClient } from "@/lib/query-client"
import { router } from "@/router"
import "@/styles/globals.css"
import sonarIcon from "../../../../build/icon.svg"

document.querySelector<HTMLLinkElement>("#sonar-icon")!.href = sonarIcon

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router
  }
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  </StrictMode>,
)
