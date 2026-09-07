import type { LucideIcon } from "lucide-react"
import { Box, KeyRound, LayoutDashboard, Layers, Play } from "lucide-react"

export type NavItem = {
  name: string
  href: string
  icon: LucideIcon
}

export type NavSection = {
  id: string
  title: string
  collapsible?: boolean
  items: NavItem[]
}

/** Product nav. Replace items; do not invent a second sidebar. */
export const APP_TITLE = "Cloudcell"

export const NAV_SECTIONS: NavSection[] = [
  {
    id: "overview",
    title: "Overview",
    items: [{ name: "Dashboard", href: "/", icon: LayoutDashboard }],
  },
  {
    id: "operate",
    title: "Operate",
    items: [
      { name: "Sandboxes", href: "/sandboxes", icon: Box },
      { name: "Snapshots", href: "/snapshots", icon: Layers },
      { name: "Playground", href: "/playground", icon: Play },
      { name: "API Keys", href: "/keys", icon: KeyRound },
    ],
  },
]
