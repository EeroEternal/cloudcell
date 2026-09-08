import { useEffect, useState } from "react"
import { useNavigate } from "react-router-dom"
import { CircleHelp, LogOut, Settings, UserCircle } from "lucide-react"
import { Avatar, AvatarFallback } from "@/components/ui/avatar"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { api, setSession, type AuthStatus } from "@/lib/api"
import { t } from "@/lib/i18n"

export function UserMenu() {
  const navigate = useNavigate()
  const [email, setEmail] = useState("")

  useEffect(() => {
    api<AuthStatus>("/api/v1/auth/status")
      .then((status) => setEmail(status.email ?? ""))
      .catch(() => setEmail(""))
  }, [])

  const signOut = async () => {
    try {
      await api("/api/v1/auth/logout", { method: "POST" })
    } catch {
      /* still clear the tab session */
    }
    setSession(null)
    navigate("/login", { replace: true })
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          aria-label={t("common.account")}
          className="rounded-full text-muted-foreground transition-all hover:text-foreground hover:ring-2 hover:ring-primary/20"
        >
          <Avatar className="h-8 w-8">
            <AvatarFallback className="rounded-lg bg-transparent">
              <UserCircle className="h-4 w-4" />
            </AvatarFallback>
          </Avatar>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent className="w-56" align="end" sideOffset={10}>
        <DropdownMenuLabel className="p-0">
          <div className="flex items-center gap-2 px-3 py-2 text-left text-sm">
            <Avatar className="h-8 w-8 text-muted-foreground">
              <AvatarFallback className="rounded-lg bg-transparent">
                <UserCircle className="h-4 w-4" />
              </AvatarFallback>
            </Avatar>
            <div className="grid min-w-0 flex-1 text-left text-sm leading-tight">
              <span className="truncate font-semibold">{email || t("common.account")}</span>
            </div>
          </div>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuGroup>
          <DropdownMenuItem className="justify-start" onClick={() => navigate("/settings")}>
            <Settings className="mr-2 h-4 w-4" />
            <span>{t("common.settings")}</span>
          </DropdownMenuItem>
          <DropdownMenuItem className="justify-start" onClick={() => navigate("/help")}>
            <CircleHelp className="mr-2 h-4 w-4" />
            <span>{t("common.help")}</span>
          </DropdownMenuItem>
        </DropdownMenuGroup>
        <DropdownMenuSeparator />
        <DropdownMenuItem
          className="justify-start text-destructive focus:text-destructive"
          onClick={() => void signOut()}
        >
          <LogOut className="mr-2 h-4 w-4" />
          <span>{t("auth.signOut")}</span>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
