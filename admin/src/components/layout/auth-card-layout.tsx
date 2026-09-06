import type { ReactNode } from "react"
import { Link } from "react-router-dom"
import { Card } from "@/components/ui/card"
import { Toaster } from "@/components/ui/sonner"
import { cn } from "@/lib/utils"
import { t, useI18n } from "@/lib/i18n"

interface AuthCardLayoutProps {
  activeTab: "register" | "login"
  title: string
  subtitle: string
  children: ReactNode
}

export function AuthCardLayout({
  activeTab,
  title,
  subtitle,
  children,
}: AuthCardLayoutProps) {
  useI18n()

  return (
    <div className="flex min-h-svh items-center justify-center bg-muted/40 p-4">
      <Toaster position="top-center" richColors />
      <div className="my-auto grid w-full max-w-4xl grid-cols-1 items-stretch gap-6 md:grid-cols-2">
        <Card className="flex flex-col justify-between border bg-card/60 p-8 shadow-sm backdrop-blur-sm">
          <div>
            <div className="flex items-center gap-3">
              <div className="flex size-10 items-center justify-center rounded-xl bg-primary text-sm font-semibold text-primary-foreground shadow-sm">
                C
              </div>
              <div>
                <div className="text-base font-bold leading-tight text-foreground">Cloudcell</div>
                <div className="text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
                  {t("auth.introBadge")}
                </div>
              </div>
            </div>
            <div className="mt-8 flex flex-col gap-3">
              <h2 className="text-2xl font-bold leading-snug tracking-tight text-foreground">
                {t("auth.introHeading")}
              </h2>
              <p className="text-xs leading-relaxed text-muted-foreground sm:text-sm">
                {t("auth.introSubheading")}
              </p>
            </div>
            <div className="mt-8 flex flex-col gap-4 border-t pt-6">
              {[1, 2, 3].map((n) => (
                <div key={n} className={n === 1 ? "flex items-start gap-3" : "flex items-start gap-3 border-t pt-4"}>
                  <span className="mt-0.5 shrink-0 font-mono text-xs font-bold text-primary">
                    0{n}
                  </span>
                  <div>
                    <div className="text-xs font-semibold text-foreground sm:text-sm">
                      {t(`auth.introPoint${n}Title`)}
                    </div>
                    <div className="mt-0.5 text-xs text-muted-foreground">
                      {t(`auth.introPoint${n}Desc`)}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
          <div className="mt-8 flex flex-wrap gap-2 border-t pt-6">
            <span className="inline-flex items-center rounded-full border bg-muted/60 px-2.5 py-1 text-xs font-medium text-muted-foreground">
              {t("auth.tagGit")}
            </span>
            <span className="inline-flex items-center rounded-full border bg-muted/60 px-2.5 py-1 text-xs font-medium text-muted-foreground">
              {t("auth.tagSession")}
            </span>
            <span className="inline-flex items-center rounded-full border bg-muted/60 px-2.5 py-1 text-xs font-medium text-muted-foreground">
              {t("auth.tagLoop")}
            </span>
          </div>
        </Card>

        <Card className="flex flex-col justify-between border bg-card p-8 shadow-sm">
          <div>
            <div className="mb-8 grid grid-cols-2 rounded-lg bg-muted p-1 text-sm font-medium">
              <Link
                to="/register"
                className={cn(
                  "rounded-md py-1.5 text-center transition-all",
                  activeTab === "register"
                    ? "bg-background font-semibold text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                {t("auth.registerTab")}
              </Link>
              <Link
                to="/login"
                className={cn(
                  "rounded-md py-1.5 text-center transition-all",
                  activeTab === "login"
                    ? "bg-background font-semibold text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground",
                )}
              >
                {t("auth.loginTab")}
              </Link>
            </div>
            <div className="mb-6 flex flex-col gap-1.5">
              <h1 className="text-2xl font-bold tracking-tight text-foreground">{title}</h1>
              <p className="text-xs leading-relaxed text-muted-foreground sm:text-sm">{subtitle}</p>
            </div>
            {children}
          </div>
          <p className="mt-6 text-center text-[11px] text-muted-foreground/70">
            {t("auth.termsNotice")}
          </p>
        </Card>
      </div>
    </div>
  )
}
