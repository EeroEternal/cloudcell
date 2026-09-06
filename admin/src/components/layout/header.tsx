import { SidebarTrigger } from "@/components/ui/sidebar"
import { Separator } from "@/components/ui/separator"
import { Button } from "@/components/ui/button"
import { api, setSession } from "@/lib/api"
import { t } from "@/lib/i18n"

export function SiteHeader() {
  const signOut = async () => {
    try {
      await api("/api/v1/auth/logout", { method: "POST" })
    } catch {
      /* still clear the tab session */
    }
    setSession(null)
    window.location.reload()
  }

  return (
    <header className="relative z-50 flex h-14 shrink-0 items-center border-b bg-background/95 px-4 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <SidebarTrigger className="-ml-1" />
      <Separator orientation="vertical" className="mx-3 hidden h-4 sm:block" />
      <div className="ml-auto">
        <Button variant="ghost" size="sm" onClick={signOut}>
          {t("auth.signOut", "Sign out")}
        </Button>
      </div>
    </header>
  )
}
