import { useState, useEffect, type FormEvent } from "react"
import { Navigate, useNavigate } from "react-router-dom"
import { ArrowRight } from "lucide-react"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { AuthCardLayout } from "@/components/layout/auth-card-layout"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { api, getSession, setSession } from "@/lib/api"
import { t, useI18n } from "@/lib/i18n"

const fieldClassName =
  "h-12 rounded-xl border-transparent bg-muted/40 px-4 transition-all focus:border-primary/20 focus:bg-background focus:ring-4 focus:ring-primary/5"

function usernameFromEmail(email: string): string {
  return email.split("@")[0]?.toLowerCase().replace(/[^a-z0-9_-]/g, "") ?? ""
}

export default function RegisterPage() {
  useI18n()
  const navigate = useNavigate()
  const [emailSent, setEmailSent] = useState(false)
  const [email, setEmail] = useState("")
  const [code, setCode] = useState("")
  const [password, setPassword] = useState("")
  const [confirm, setConfirm] = useState("")
  const [sendingCode, setSendingCode] = useState(false)
  const [countdown, setCountdown] = useState(0)
  const [pending, setPending] = useState(false)

  useEffect(() => {
    if (countdown <= 0) return
    const timer = setTimeout(() => setCountdown(countdown - 1), 1000)
    return () => clearTimeout(timer)
  }, [countdown])

  if (getSession()) return <Navigate to="/" replace />

  async function onSendCode(e?: FormEvent) {
    if (e) e.preventDefault()
    const trimmed = email.trim()
    if (!trimmed || !trimmed.includes("@") || trimmed.includes(" ")) {
      toast.error(t("auth.email"))
      return
    }
    setSendingCode(true)
    try {
      await api<{ ok: boolean }>("/api/v1/auth/send-code", {
        method: "POST",
        body: JSON.stringify({ email: trimmed }),
      })
      toast.success(t("auth.codeSent"))
      setCountdown(60)
      setEmailSent(true)
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t("common.error"))
    } finally {
      setSendingCode(false)
    }
  }

  async function onSubmit(e: FormEvent) {
    e.preventDefault()
    if (code.trim().length !== 6) {
      toast.error(t("auth.codeInvalid"))
      return
    }
    if (password.length < 8) {
      toast.error(t("auth.passwordTooShort"))
      return
    }
    if (password !== confirm) {
      toast.error(t("auth.passwordMismatch"))
      return
    }
    setPending(true)
    try {
      const res = await api<{ ok: boolean; token?: string }>("/api/v1/auth/register", {
        method: "POST",
        body: JSON.stringify({
          email: email.trim(),
          code: code.trim(),
          username: usernameFromEmail(email.trim()) || undefined,
          password,
        }),
      })
      if (res.token) {
        setSession(res.token)
        toast.success(t("auth.registerOk"))
        navigate("/", { replace: true })
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t("common.error"))
    } finally {
      setPending(false)
    }
  }

  const sendLabel = countdown > 0 ? `${countdown}s` : t("auth.sendCode")

  return (
    <AuthCardLayout activeTab="register" title={t("auth.registerTitle")}>
      <div className="min-h-[320px]">
        {!emailSent ? (
          <form className="space-y-6" onSubmit={onSendCode}>
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
            <Button
              type="submit"
              className="h-12 w-full gap-2 text-base font-semibold"
              disabled={sendingCode || !email.trim()}
            >
              {sendingCode ? t("common.loading") : t("auth.sendCode")}
              {!sendingCode && <ArrowRight className="h-5 w-5" />}
            </Button>
          </form>
        ) : (
          <form className="space-y-6" onSubmit={onSubmit}>
            <div className="flex items-center justify-between rounded-xl bg-muted/40 px-4 py-3 text-sm text-muted-foreground">
              <span className="truncate">{email}</span>
              <button
                type="button"
                className="ml-2 shrink-0 text-xs font-semibold text-primary underline-offset-4 hover:underline"
                onClick={() => {
                  setEmailSent(false)
                  setCode("")
                  setPassword("")
                  setConfirm("")
                }}
              >
                {t("auth.changeEmail")}
              </button>
            </div>

            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <Label htmlFor="code" className="ml-1 text-sm font-semibold text-foreground/80">
                  {t("auth.code")}
                </Label>
                <button
                  type="button"
                  className="text-xs font-semibold text-primary underline-offset-4 hover:underline disabled:no-underline disabled:opacity-50"
                  disabled={sendingCode || countdown > 0}
                  onClick={() => void onSendCode()}
                >
                  {sendingCode ? t("common.loading") : sendLabel}
                </button>
              </div>
              <Input
                id="code"
                name="code"
                type="text"
                inputMode="numeric"
                pattern="[0-9]*"
                maxLength={6}
                autoComplete="one-time-code"
                placeholder={t("auth.codePlaceholder")}
                className={fieldClassName}
                value={code}
                onChange={(e) => setCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
                required
                autoFocus
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="password" className="ml-1 text-sm font-semibold text-foreground/80">
                {t("auth.password")}
              </Label>
              <Input
                id="password"
                name="password"
                type="password"
                autoComplete="new-password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                placeholder={t("auth.passwordPlaceholder")}
                className={fieldClassName}
                minLength={8}
                required
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="confirm" className="ml-1 text-sm font-semibold text-foreground/80">
                {t("auth.confirmPassword")}
              </Label>
              <Input
                id="confirm"
                name="confirm-password"
                type="password"
                autoComplete="new-password"
                value={confirm}
                onChange={(e) => setConfirm(e.target.value)}
                placeholder={t("auth.confirmPasswordPlaceholder")}
                className={fieldClassName}
                minLength={8}
                required
              />
            </div>

            <Button type="submit" className="h-12 w-full gap-2 text-base font-semibold" disabled={pending}>
              {pending ? t("common.loading") : t("auth.register")}
              {!pending && <ArrowRight className="h-5 w-5" />}
            </Button>
          </form>
        )}
      </div>
    </AuthCardLayout>
  )
}
