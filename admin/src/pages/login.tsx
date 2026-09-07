import { useState, type FormEvent } from "react"
import { Navigate, useNavigate } from "react-router-dom"
import { Eye, EyeOff, ArrowRight } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { AuthCardLayout } from "@/components/layout/auth-card-layout"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { api, getSession, setSession } from "@/lib/api"
import { t, useI18n } from "@/lib/i18n"

const fieldClassName =
  "h-12 rounded-xl border-transparent bg-muted/40 px-4 transition-all focus:border-primary/20 focus:bg-background focus:ring-4 focus:ring-primary/5"

export default function LoginPage() {
  useI18n()
  const navigate = useNavigate()
  const [email, setEmail] = useState("")
  const [password, setPassword] = useState("")
  const [showPassword, setShowPassword] = useState(false)
  const [pending, setPending] = useState(false)

  if (getSession()) return <Navigate to="/" replace />

  async function onSubmit(e: FormEvent) {
    e.preventDefault()
    setPending(true)
    try {
      const res = await api<{ token: string }>("/api/v1/auth/login", {
        method: "POST",
        body: JSON.stringify({ email: email.trim(), password }),
      })
      setSession(res.token)
      navigate("/", { replace: true })
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t("common.error"))
    } finally {
      setPending(false)
    }
  }

  return (
    <AuthCardLayout activeTab="login" title={t("auth.loginTitle")}>
      <form className="space-y-6" onSubmit={onSubmit}>
        <div className="space-y-2">
          <Label htmlFor="email" className="ml-1 text-sm font-semibold text-foreground/80">
            {t("auth.email")}
          </Label>
          <Input
            id="email"
            name="email"
            type="email"
            inputMode="email"
            autoComplete="username"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder={t("auth.emailPlaceholder")}
            className={fieldClassName}
            required
            autoFocus
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="password" className="text-sm font-semibold text-foreground/80">
            {t("auth.password")}
          </Label>
          <div className="relative">
            <Input
              id="password"
              name="password"
              type={showPassword ? "text" : "password"}
              autoComplete="current-password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder={t("auth.passwordPlaceholder")}
              className={`${fieldClassName} pr-12`}
              required
            />
            <button
              type="button"
              onClick={() => setShowPassword((open) => !open)}
              className="absolute right-3.5 top-1/2 -translate-y-1/2 p-1 text-muted-foreground/50 transition-colors hover:text-foreground"
              aria-label={showPassword ? t("auth.hidePassword") : t("auth.showPassword")}
            >
              {showPassword ? <EyeOff className="h-5 w-5" /> : <Eye className="h-5 w-5" />}
            </button>
          </div>
        </div>
        <Button type="submit" className="h-12 w-full gap-2 text-base font-semibold" disabled={pending}>
          {pending ? t("common.loading") : t("auth.login")}
          {!pending && <ArrowRight className="h-5 w-5" />}
        </Button>
      </form>
    </AuthCardLayout>
  )
}
