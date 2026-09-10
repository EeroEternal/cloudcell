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
    title: "nav.overview",
    items: [{ name: "nav.dashboard", href: "/", icon: LayoutDashboard }],
  },
  {
    id: "operate",
    title: "nav.operate",
    items: [
      { name: "nav.sandboxes", href: "/sandboxes", icon: Box },
      { name: "nav.snapshots", href: "/snapshots", icon: Layers },
      { name: "nav.playground", href: "/playground", icon: Play },
      { name: "nav.keys", href: "/keys", icon: KeyRound },
    ],
  },
]
