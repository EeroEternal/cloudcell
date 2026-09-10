import type { ReactNode } from "react"
import { Link } from "react-router-dom"
import { LanguageSwitcher } from "@/components/layout/language-switcher"
import { Toaster } from "@/components/ui/sonner"
import { t, useI18n } from "@/lib/i18n"

const revealStyle = (delayMs: number) => ({
  animationDelay: `${delayMs}ms`,
  animationFillMode: "both" as const,
})

interface AuthCardLayoutProps {
  activeTab: "register" | "login"
  title: string
  children: ReactNode
}

function BrandMark() {
  return (
    <div className="flex size-10 items-center justify-center rounded-lg bg-primary text-sm font-semibold text-primary-foreground">
      C
    </div>
  )
}

export function AuthCardLayout({ activeTab, title, children }: AuthCardLayoutProps) {
  useI18n()
  const loginStats = [
    { value: "~5ms", label: t("auth.statStart") },
    { value: "0", label: t("auth.statDocker") },
    { value: "LSM", label: t("auth.statKernel") },
  ]

  return (
    <div className="flex min-h-screen overflow-hidden bg-background">
      <div className="absolute right-4 top-4 z-50 flex items-center gap-3">
        <Link
          to="/help"
          className="text-sm text-muted-foreground underline-offset-4 hover:text-foreground hover:underline"
        >
          {t("auth.agentGuide")}
        </Link>
        <LanguageSwitcher />
      </div>

      <div className="relative hidden items-center justify-center overflow-hidden bg-muted/30 lg:flex lg:w-1/2">
        <div className="absolute inset-0 z-0">
          <div className="absolute right-[-10%] top-[10%] h-[60%] w-[60%] rounded-full bg-primary/10 blur-[120px]" />
          <div className="absolute bottom-[-10%] left-[-10%] h-[40%] w-[40%] rounded-full bg-accent/5 blur-[100px]" />
        </div>

        <div className="relative z-10 px-12 xl:px-24">
          <div
            className="mb-10 flex items-center gap-3 duration-700 animate-in fade-in slide-in-from-bottom-5"
            style={revealStyle(0)}
          >
            <BrandMark />
            <span className="text-2xl font-semibold tracking-tight text-foreground/90">
              {t("auth.productName")}
            </span>
          </div>

          <h1
            className="mb-4 text-4xl font-semibold leading-tight text-foreground duration-700 animate-in fade-in slide-in-from-bottom-5 xl:text-5xl"
            style={revealStyle(100)}
          >
            {t("auth.brandingTitle")} <br />
            <span className="text-primary">{t("auth.brandingSubtitle")}</span>
          </h1>

          <p
            className="mb-16 max-w-lg text-lg leading-relaxed text-muted-foreground/80 duration-700 animate-in fade-in slide-in-from-bottom-5"
            style={revealStyle(200)}
          >
            {t("auth.brandingDescription")}
          </p>

          <div
            className="flex gap-12 duration-700 animate-in fade-in slide-in-from-bottom-5"
            style={revealStyle(300)}
          >
            {loginStats.map((item) => (
              <div key={item.label} className="flex flex-col gap-1">
                <div className="text-3xl font-semibold text-foreground">{item.value}</div>
                <div className="text-sm text-muted-foreground/60">{item.label}</div>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="relative flex flex-1 items-center justify-center overflow-y-auto bg-background p-6">
        <div className="w-full max-w-[440px]">
          <div className="rounded-lg border border-border bg-card p-10 shadow-sm lg:p-12">
            <div className="duration-300 animate-in fade-in slide-in-from-right-4">
              <div className="mb-10 text-center">
                <h2 className="text-page-title text-foreground">{title}</h2>
              </div>
              {children}
            </div>

            <div className="mt-8 text-center">
              <p className="text-sm text-muted-foreground/70">
                {activeTab === "login" ? t("auth.noAccount") : t("auth.hasAccount")}
                <Link
                  to={activeTab === "login" ? "/register" : "/login"}
                  className="ml-2 font-semibold text-primary underline-offset-4 hover:underline"
                >
                  {activeTab === "login" ? t("auth.registerNow") : t("auth.loginNow")}
                </Link>
              </p>
            </div>
          </div>
        </div>
      </div>
      <Toaster position="top-center" richColors />
    </div>
  )
}
