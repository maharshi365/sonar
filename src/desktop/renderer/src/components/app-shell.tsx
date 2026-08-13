import type { CSSProperties, ReactNode } from "react"
import { Link } from "@tanstack/react-router"
import { AudioLines, Box, History, Settings } from "lucide-react"

import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarHeader,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarRail,
  SidebarTrigger,
} from "@/components/ui/sidebar"
import { TooltipProvider } from "@/components/ui/tooltip"

const navigation = [
  { label: "Transcribe", to: "/", icon: AudioLines, exact: true },
  { label: "History", to: "/history", icon: History },
  { label: "Models", to: "/models", icon: Box },
] as const

export function AppShell({ children }: { children: ReactNode }) {
  return (
    <TooltipProvider>
      <SidebarProvider
        style={{
          "--sidebar-width": "14rem",
        } as CSSProperties}
      >
        <Sidebar collapsible="icon">
          <SidebarHeader className="h-16 flex-row items-center px-3">
            <div className="flex min-w-0 flex-1 items-center gap-3 overflow-hidden px-1 group-data-[collapsible=icon]:hidden">
              <div className="grid size-8 shrink-0 place-items-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground shadow-[0_0_24px_-6px_var(--sidebar-primary)]">
                <AudioLines className="size-4" />
              </div>
              <span className="truncate text-sm font-semibold tracking-wide">
                SONAR
              </span>
            </div>
            <SidebarTrigger className="shrink-0" />
          </SidebarHeader>

          <SidebarContent>
            <SidebarGroup>
              <SidebarGroupContent>
                <SidebarMenu aria-label="Primary navigation">
                  {navigation.map((item) => (
                    <SidebarMenuItem key={item.to}>
                      <SidebarMenuButton
                        tooltip={item.label}
                        render={
                          <Link
                            to={item.to}
                            activeOptions={{ exact: "exact" in item && item.exact }}
                            activeProps={{ "data-active": true }}
                          />
                        }
                      >
                        <item.icon />
                        <span>{item.label}</span>
                      </SidebarMenuButton>
                    </SidebarMenuItem>
                  ))}
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>
          </SidebarContent>

          <SidebarFooter>
            <SidebarMenu>
              <SidebarMenuItem>
                <SidebarMenuButton
                  tooltip="Settings"
                  render={<Link to="/settings" activeProps={{ "data-active": true }} />}
                >
                  <Settings />
                  <span>Settings</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarFooter>
          <SidebarRail />
        </Sidebar>

        <SidebarInset className="min-w-0">
          <main className="h-screen overflow-y-auto">{children}</main>
        </SidebarInset>
      </SidebarProvider>
    </TooltipProvider>
  )
}
