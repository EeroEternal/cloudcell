import { useEffect, useState } from "react"
import { Link } from "react-router-dom"
import Markdown from "react-markdown"
import remarkGfm from "remark-gfm"
import { Languages } from "lucide-react"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { getSession } from "@/lib/api"
import { t, useI18n, type Language } from "@/lib/i18n"

type DocId = "guide" | "changelog"

const DOCS: { id: DocId; href: string }[] = [
  { id: "guide", href: "/docs/guide.md" },
  { id: "changelog", href: "/docs/changelog.md" },
]

function LanguageSwitcher() {
  const { language, setLanguage } = useI18n()
  const languages: { code: Language; name: string }[] = [
    { code: "zh", name: t("common.chinese") },
    { code: "en", name: t("common.english") },
  ]
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          size="icon-sm"
          className="text-muted-foreground hover:text-foreground"
          aria-label={t("common.language")}
        >
          <Languages aria-hidden="true" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="z-50">
        {languages.map((lang) => (
          <DropdownMenuItem
            key={lang.code}
            onClick={() => setLanguage(lang.code)}
            className={language === lang.code ? "bg-accent" : ""}
          >
            {lang.name}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

export default function HelpPage() {
  useI18n()
  const [docId, setDocId] = useState<DocId>("guide")
  const [loaded, setLoaded] = useState<{ href: string; text: string } | null>(null)
  const [error, setError] = useState("")
  const signedIn = Boolean(getSession())
  const active = DOCS.find((d) => d.id === docId) ?? DOCS[0]
  const source = loaded?.href === active.href ? loaded.text : ""

  useEffect(() => {
    let cancelled = false
    fetch(active.href)
      .then((res) => {
        if (!res.ok) throw new Error(res.statusText)
        return res.text()
      })
      .then((text) => {
        if (cancelled) return
        setError("")
        setLoaded({ href: active.href, text })
      })
      .catch((err: Error) => {
        if (cancelled) return
        setError(err.message || t("common.error"))
      })
    return () => {
      cancelled = true
    }
  }, [active.href])

  return (
    <div className="flex min-h-screen flex-col bg-background">
      <header className="flex h-14 items-center gap-3 border-b border-border px-8">
        <Link to={signedIn ? "/" : "/login"} className="text-sm font-semibold text-foreground">
          {t("auth.productName")}
        </Link>
        <span className="text-sm text-muted-foreground">{t("help.title")}</span>
        <div className="ml-auto flex items-center gap-2">
          <LanguageSwitcher />
          <Button variant="outline" size="sm" asChild>
            <Link to={signedIn ? "/" : "/login"}>
              {signedIn ? t("help.console") : t("auth.login")}
            </Link>
          </Button>
        </div>
      </header>
      <div className="flex min-h-0 flex-1 flex-col gap-6 px-8 py-8 xl:flex-row">
        <nav className="w-full shrink-0 xl:w-56">
          <ul className="flex flex-col gap-1">
            {DOCS.map((doc) => (
              <li key={doc.id}>
                <button
                  type="button"
                  onClick={() => setDocId(doc.id)}
                  className={`flex w-full rounded-md px-3 py-2 text-left text-sm leading-5 ${
                    doc.id === docId
                      ? "bg-primary/10 font-medium text-primary"
                      : "hover:bg-muted"
                  }`}
                >
                  {t(`help.${doc.id}`)}
                </button>
              </li>
            ))}
          </ul>
          <a
            href={active.href}
            className="mt-4 inline-block px-3 text-sm text-muted-foreground underline-offset-4 hover:text-foreground hover:underline"
          >
            {t("help.raw")}
          </a>
        </nav>
        <Card className="min-h-0 min-w-0 flex-1 overflow-y-auto p-6">
          {error ? (
            <p className="text-sm text-destructive">{error}</p>
          ) : !source ? (
            <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
          ) : (
            <div
              className={[
                "max-w-3xl space-y-4 text-sm leading-6 text-foreground",
                "[&_h1]:text-page-title [&_h1]:text-foreground",
                "[&_h2]:pt-4 [&_h2]:text-section-title [&_h2]:text-foreground",
                "[&_h3]:pt-2 [&_h3]:font-semibold",
                "[&_p]:text-foreground/90",
                "[&_a]:text-primary [&_a]:underline-offset-4 hover:[&_a]:underline",
                "[&_ul]:list-disc [&_ul]:space-y-1 [&_ul]:pl-5",
                "[&_ol]:list-decimal [&_ol]:space-y-1 [&_ol]:pl-5",
                "[&_table]:w-full [&_table]:text-left [&_th]:border-b [&_th]:border-border [&_th]:py-2 [&_th]:pr-3 [&_th]:font-medium",
                "[&_td]:border-b [&_td]:border-border [&_td]:py-2 [&_td]:pr-3",
                "[&_code]:rounded-sm [&_code]:bg-muted [&_code]:px-1 [&_code]:py-0.5 [&_code]:text-xs",
                "[&_pre]:overflow-x-auto [&_pre]:rounded-md [&_pre]:border [&_pre]:border-border [&_pre]:bg-muted/40 [&_pre]:p-4",
                "[&_pre_code]:bg-transparent [&_pre_code]:p-0",
              ].join(" ")}
            >
              <Markdown remarkPlugins={[remarkGfm]}>{source}</Markdown>
            </div>
          )}
        </Card>
      </div>
    </div>
  )
}
