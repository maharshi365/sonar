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
        <Sidebar collapsible="icon" className="pt-11">
          <SidebarHeader className="h-16 justify-center px-3">
            <div className="flex items-center gap-3 overflow-hidden px-1 group-data-[collapsible=icon]:justify-center group-data-[collapsible=icon]:px-0">
              <div className="grid size-8 shrink-0 place-items-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground shadow-[0_0_24px_-6px_var(--sidebar-primary)]">
                <AudioLines className="size-4" />
              </div>
              <span className="truncate text-sm font-semibold tracking-wide group-data-[collapsible=icon]:hidden">
                SONAR
              </span>
            </div>
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

        <SidebarInset className="min-w-0 pt-11">
          <div className="fixed inset-x-0 top-0 z-50 h-11 border-b border-border/70 bg-background/90 [-webkit-app-region:drag]">
            <SidebarTrigger className="absolute left-2 top-2" />
          </div>
          <main className="h-[calc(100vh-2.75rem)] overflow-y-auto">{children}</main>
        </SidebarInset>
      </SidebarProvider>
    </TooltipProvider>
  )
}
