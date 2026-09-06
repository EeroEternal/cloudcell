import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { api, setSession, type AuthResponse, type AuthStatus } from "@/lib/api"
import { t } from "@/lib/i18n"

export default function AuthScreen({
  status,
  onAuthenticated,
}: {
  status: AuthStatus
  onAuthenticated: () => void
}) {
  const [mode, setMode] = useState<"login" | "register">(
    status.has_users ? "login" : "register",
  )
  const [email, setEmail] = useState("")
  const [password, setPassword] = useState("")
  const [error, setError] = useState("")
  const [saving, setSaving] = useState(false)

  const showRegister = !status.has_users || status.registration_enabled

  const submit = async () => {
    setSaving(true)
    setError("")
    try {
      const path = mode === "register" ? "/api/v1/auth/register" : "/api/v1/auth/login"
      const result = await api<AuthResponse>(path, {
        method: "POST",
        body: JSON.stringify({ email, password }),
      })
      setSession(result.token)
      onAuthenticated()
    } catch (err) {
      setError(err instanceof Error ? err.message : "auth failed")
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="flex min-h-svh items-center justify-center bg-muted/40 p-4">
      <Card className="w-full max-w-md p-6">
        <h1 className="text-lg font-semibold">
          {mode === "register"
            ? t("auth.registerTitle", "Create account")
            : t("auth.loginTitle", "Sign in")}
        </h1>
        <div className="mt-4 flex flex-col gap-3">
          <div className="flex flex-col gap-2">
            <Label htmlFor="email">{t("auth.email", "Email")}</Label>
            <Input
              id="email"
              type="email"
              autoComplete="username"
              value={email}
              onChange={(event) => setEmail(event.target.value)}
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="password">{t("auth.password", "Password")}</Label>
            <Input
              id="password"
              type="password"
              autoComplete={mode === "register" ? "new-password" : "current-password"}
              value={password}
              onChange={(event) => setPassword(event.target.value)}
            />
          </div>
          {error ? <p className="text-sm text-destructive">{error}</p> : null}
          <Button onClick={submit} disabled={saving || !email || password.length < 8}>
            {mode === "register"
              ? t("auth.register", "Create account")
              : t("auth.login", "Sign in")}
          </Button>
          {showRegister && status.has_users ? (
            <Button
              variant="ghost"
              type="button"
              onClick={() => setMode(mode === "login" ? "register" : "login")}
            >
              {mode === "login"
                ? t("auth.switchRegister", "Create an account")
                : t("auth.switchLogin", "Sign in instead")}
            </Button>
          ) : null}
        </div>
      </Card>
    </div>
  )
}
