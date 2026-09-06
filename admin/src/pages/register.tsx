import { useState, useEffect, type FormEvent } from "react"
import { Navigate, useNavigate } from "react-router-dom"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { AuthCardLayout } from "@/components/layout/auth-card-layout"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { api, getSession, setSession } from "@/lib/api"
import { t, useI18n } from "@/lib/i18n"

type RegisterStep = "email" | "code" | "credentials"

export default function RegisterPage() {
  useI18n()
  const navigate = useNavigate()
  const [step, setStep] = useState<RegisterStep>("email")
  const [email, setEmail] = useState("")
  const [code, setCode] = useState("")
  const [username, setUsername] = useState("")
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
      setStep("code")
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t("common.error"))
    } finally {
      setSendingCode(false)
    }
  }

  async function onVerifyCode(e: FormEvent) {
    e.preventDefault()
    const trimmedCode = code.trim()
    if (trimmedCode.length !== 6) {
      toast.error(t("auth.codeInvalid"))
      return
    }
    setPending(true)
    try {
      await api<{ ok: boolean }>("/api/v1/auth/verify-code", {
        method: "POST",
        body: JSON.stringify({
          email: email.trim(),
          code: trimmedCode,
        }),
      })
      if (!username.trim()) {
        const defaultName = email
          .split("@")[0]
          .toLowerCase()
          .replace(/[^a-z0-9_-]/g, "")
        setUsername(defaultName)
      }
      setStep("credentials")
    } catch (err) {
      toast.error(err instanceof Error ? err.message : t("common.error"))
    } finally {
      setPending(false)
    }
  }

  async function onCompleteRegister(e: FormEvent) {
    e.preventDefault()
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
          username: username.trim() || undefined,
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

  return (
    <AuthCardLayout
      activeTab="register"
      title={t("auth.workspaceRegisterTitle")}
      subtitle={t("auth.workspaceRegisterDesc")}
    >
      {step === "email" && (
        <form className="flex flex-col gap-4" onSubmit={onSendCode}>
          <div className="flex flex-col gap-2">
            <Label htmlFor="email">{t("auth.email")}</Label>
            <Input
              id="email"
              name="email"
              type="email"
              inputMode="email"
              autoComplete="username"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="you@company.com"
              required
              autoFocus
            />
          </div>
          <Button type="submit" className="w-full" disabled={sendingCode || !email.trim()}>
            {sendingCode ? "..." : t("auth.sendCode")}
          </Button>
        </form>
      )}

      {step === "code" && (
        <form className="flex flex-col gap-4" onSubmit={onVerifyCode}>
          <div className="flex items-center justify-between rounded border bg-muted/50 p-2.5 text-sm text-muted-foreground">
            <span className="truncate">{email}</span>
            <button
              type="button"
              className="ml-2 shrink-0 text-xs text-primary underline"
              onClick={() => setStep("email")}
            >
              {t("auth.back")}
            </button>
          </div>
          <div className="flex flex-col gap-2">
            <div className="flex items-center justify-between">
              <Label htmlFor="code">{t("auth.code")}</Label>
              <button
                type="button"
                className="text-xs text-primary underline disabled:no-underline disabled:opacity-50"
                disabled={sendingCode || countdown > 0}
                onClick={() => void onSendCode()}
              >
                {countdown > 0 ? `${countdown}s` : t("auth.resendCode")}
              </button>
            </div>
            <Input
              id="code"
              name="code"
              type="text"
              inputMode="numeric"
              pattern="[0-9]*"
              maxLength={6}
              placeholder="123456"
              className="text-center font-mono text-lg tracking-widest"
              value={code}
              onChange={(e) => setCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
              required
              autoFocus
            />
          </div>
          <div className="flex gap-2 pt-2">
            <Button type="button" variant="outline" className="w-1/3" onClick={() => setStep("email")}>
              {t("auth.back")}
            </Button>
            <Button type="submit" className="flex-1" disabled={pending || code.trim().length !== 6}>
              {pending ? "..." : t("auth.next")}
            </Button>
          </div>
        </form>
      )}

      {step === "credentials" && (
        <form className="flex flex-col gap-4" onSubmit={onCompleteRegister}>
          <div className="flex flex-col gap-2">
            <Label htmlFor="username">{t("auth.username")}</Label>
            <Input
              id="username"
              name="username"
              type="text"
              autoComplete="username"
              placeholder={t("auth.usernamePlaceholder")}
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              required
              autoFocus
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="password">{t("auth.password")}</Label>
            <Input
              id="password"
              name="password"
              type="password"
              autoComplete="new-password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              minLength={8}
              required
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="confirm">{t("auth.confirmPassword")}</Label>
            <Input
              id="confirm"
              name="confirm-password"
              type="password"
              autoComplete="new-password"
              value={confirm}
              onChange={(e) => setConfirm(e.target.value)}
              minLength={8}
              required
            />
          </div>
          <div className="flex gap-2 pt-2">
            <Button type="button" variant="outline" className="w-1/3" onClick={() => setStep("code")}>
              {t("auth.back")}
            </Button>
            <Button type="submit" className="flex-1" disabled={pending}>
              {pending ? "..." : t("auth.completeRegister")}
            </Button>
          </div>
        </form>
      )}
    </AuthCardLayout>
  )
}
