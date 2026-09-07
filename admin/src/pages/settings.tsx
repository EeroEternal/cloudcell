import { useEffect, useState } from "react"
import { Globe, Sliders, RotateCcw, ShieldCheck, Shield, KeyRound } from "lucide-react"
import { SectionCard } from "@/common/section-card"
import { PageContainer } from "@/components/layout/page-container"
import { PageHeader } from "@/components/layout/page-header"
import { PageShell } from "@/components/layout/page-shell"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { api, getApiKey, setApiKey } from "@/lib/api"
import { t } from "@/lib/i18n"
import { toast } from "sonner"
import { SettingsSectionNav, type SettingsSection } from "@/components/settings/SettingsSectionNav"
import { SettingsToggleRow } from "@/components/settings/SettingsToggleRow"
import { SettingsSaveBar } from "@/components/settings/SettingsSaveBar"

interface GeneralConfig {
  siteTitle: string
  maintenanceMode: boolean
  registrationEnabled: boolean
  rateLimitPerMinute: number
}

const INITIAL_CONFIG: GeneralConfig = {
  siteTitle: "Cloudcell",
  maintenanceMode: false,
  registrationEnabled: true,
  rateLimitPerMinute: 60,
}

const SECTIONS: SettingsSection[] = [
  { id: "general", label: "General", icon: Globe },
  { id: "security", label: "Security & Policy", icon: Shield },
  { id: "api-tokens", label: "Service Credentials", icon: KeyRound },
]

export default function SettingsPage() {
  const [activeSection, setActiveSection] = useState("general")
  const [config, setConfig] = useState<GeneralConfig>(INITIAL_CONFIG)
  const [draftConfig, setDraftConfig] = useState<GeneralConfig>(INITIAL_CONFIG)
  const [isEditing, setIsEditing] = useState(false)
  const [saving, setSaving] = useState(false)
  const [saveMessage, setSaveMessage] = useState<{ type: "success" | "error"; text: string } | null>(null)

  useEffect(() => {
    api<{ registration_enabled: boolean }>("/api/v1/settings")
      .then((settings) => {
        const next = { ...INITIAL_CONFIG, registrationEnabled: settings.registration_enabled }
        setConfig(next)
        setDraftConfig(next)
      })
      .catch(() => {})
  }, [])

  const handleStartEdit = () => {
    setDraftConfig(config)
    setIsEditing(true)
    setSaveMessage(null)
  }

  const handleCancelEdit = () => {
    setDraftConfig(config)
    setIsEditing(false)
    setSaveMessage(null)
  }

  const handleSave = () => {
    setSaving(true)
    api("/api/v1/settings", {
      method: "PUT",
      body: JSON.stringify({ registration_enabled: draftConfig.registrationEnabled }),
    })
      .then(() => {
        setConfig(draftConfig)
        setIsEditing(false)
        setSaveMessage({ type: "success", text: t("settings.saved", "Settings saved.") })
        setTimeout(() => setSaveMessage(null), 3000)
      })
      .catch((err: Error) => {
        setSaveMessage({ type: "error", text: err.message })
      })
      .finally(() => setSaving(false))
  }

  const handleSectionChange = (sectionId: string) => {
    setIsEditing(false)
    setActiveSection(sectionId)
    setSaveMessage(null)
  }

  return (
    <PageShell className="overflow-y-auto">
      <PageContainer className="max-w-[1100px] pb-8 gap-4">
        <PageHeader title="Settings" className="mb-0" />

        <div className="flex flex-col gap-4">
          <SettingsSectionNav
            sections={SECTIONS}
            activeSection={activeSection}
            onSectionChange={handleSectionChange}
          />

          <div className="w-full space-y-4">
            {activeSection === "general" && (
              <SectionCard
                title="General Configuration"
                description={
                  !isEditing
                    ? "Read-only overview of current system parameters."
                    : "Configure global system parameters and operational defaults."
                }
                headerExtra={
                  <div className="flex items-center gap-2">
                    {!isEditing ? (
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={handleStartEdit}
                        className="h-8 gap-1.5 text-xs font-medium"
                      >
                        <Sliders className="h-3.5 w-3.5 text-primary" />
                        Edit Settings
                      </Button>
                    ) : (
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={handleCancelEdit}
                        className="h-8 gap-1.5 text-xs"
                      >
                        <RotateCcw className="h-3.5 w-3.5" />
                        Cancel Edit
                      </Button>
                    )}
                    <div className="flex items-center gap-1.5 rounded-full border border-success/20 bg-success/10 px-2 py-0.5 text-[11px] font-medium text-success">
                      <div className="h-1.5 w-1.5 rounded-full bg-success" />
                      Active
                    </div>
                  </div>
                }
              >
                {!isEditing ? (
                  /* READ-ONLY / DISPLAY MODE */
                  <div className="space-y-4">
                    <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
                      <div className="flex flex-col justify-between rounded-lg border border-border/70 bg-card p-3.5 shadow-sm">
                        <div className="flex items-center justify-between text-xs text-muted-foreground">
                          <span className="font-medium">Site Title</span>
                          <Globe className="h-3.5 w-3.5 text-muted-foreground" />
                        </div>
                        <div className="mt-2 text-sm font-semibold text-foreground truncate">
                          {config.siteTitle}
                        </div>
                      </div>

                      <div className="flex flex-col justify-between rounded-lg border border-border/70 bg-card p-3.5 shadow-sm">
                        <div className="flex items-center justify-between text-xs text-muted-foreground">
                          <span className="font-medium">Registration</span>
                          <ShieldCheck className="h-3.5 w-3.5 text-muted-foreground" />
                        </div>
                        <div className="mt-2 text-sm font-semibold text-foreground">
                          {config.registrationEnabled ? "Enabled" : "Disabled"}
                        </div>
                      </div>

                      <div className="flex flex-col justify-between rounded-lg border border-border/70 bg-card p-3.5 shadow-sm">
                        <div className="flex items-center justify-between text-xs text-muted-foreground">
                          <span className="font-medium">Rate Limit</span>
                          <Shield className="h-3.5 w-3.5 text-muted-foreground" />
                        </div>
                        <div className="mt-2 text-sm font-semibold text-foreground">
                          {config.rateLimitPerMinute} req / min
                        </div>
                      </div>
                    </div>
                  </div>
                ) : (
                  /* EDIT MODE */
                  <div className="space-y-4">
                    <div className="grid gap-4 sm:grid-cols-2">
                      <div className="space-y-2">
                        <Label htmlFor="siteTitle" className="text-xs font-medium">Site Title</Label>
                        <Input
                          id="siteTitle"
                          value={draftConfig.siteTitle}
                          onChange={(e) =>
                            setDraftConfig({ ...draftConfig, siteTitle: e.target.value })
                          }
                          className="h-9 text-sm"
                        />
                      </div>

                      <div className="space-y-2">
                        <Label htmlFor="rateLimit" className="text-xs font-medium">Rate Limit (req/min)</Label>
                        <Input
                          id="rateLimit"
                          type="number"
                          value={draftConfig.rateLimitPerMinute}
                          onChange={(e) =>
                            setDraftConfig({
                              ...draftConfig,
                              rateLimitPerMinute: Number(e.target.value) || 0,
                            })
                          }
                          className="h-9 text-sm font-mono"
                        />
                      </div>
                    </div>

                    <div className="pt-2">
                      <SettingsToggleRow
                        label="User Registration"
                        description="Allow new operators and developers to register self-service accounts."
                        checked={draftConfig.registrationEnabled}
                        onCheckedChange={(checked) =>
                          setDraftConfig({ ...draftConfig, registrationEnabled: checked })
                        }
                      />
                    </div>
                  </div>
                )}
              </SectionCard>
            )}

            {activeSection === "security" && (
              <SectionCard title="Security & Policy" description="System-wide security safeguards and authentication posture.">
                <div className="space-y-3">
                  <SettingsToggleRow
                    label="Maintenance Mode"
                    description="Temporarily pause public data ingress while keeping the admin console accessible."
                    checked={config.maintenanceMode}
                    onCheckedChange={(checked) => setConfig({ ...config, maintenanceMode: checked })}
                  />
                </div>
              </SectionCard>
            )}

            {activeSection === "api-tokens" && <ConsoleKeySection />}

            {isEditing && (
              <SettingsSaveBar
                saving={saving}
                message={saveMessage}
                onReset={handleCancelEdit}
                onSave={handleSave}
              />
            )}
          </div>
        </div>
      </PageContainer>
    </PageShell>
  )
}

function maskKey(key: string): string {
  if (!key) return ""
  if (key.length <= 12) return "••••••••"
  return `${key.slice(0, 8)}••••`
}

function ConsoleKeySection() {
  const stored = getApiKey()
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState(stored)
  const [saving, setSaving] = useState(false)
  const [saveMessage, setSaveMessage] = useState<{
    type: "success" | "error"
    text: string
  } | null>(null)

  const startEdit = () => {
    setDraft(stored)
    setEditing(true)
    setSaveMessage(null)
  }

  const cancelEdit = () => {
    setDraft(stored)
    setEditing(false)
    setSaveMessage(null)
  }

  const save = () => {
    setSaving(true)
    const next = draft.trim()
    setApiKey(next || null)
    setEditing(false)
    setSaving(false)
    setSaveMessage({
      type: "success",
      text: next
        ? t("settings.keySaved", "API key stored for this tab")
        : t("settings.keyCleared", "API key cleared"),
    })
    toast.success(
      next
        ? t("settings.keySaved", "API key stored for this tab")
        : t("settings.keyCleared", "API key cleared"),
    )
    setTimeout(() => setSaveMessage(null), 3000)
  }

  return (
    <SectionCard
      title={t("settings.consoleKey", "Console API key")}
      headerExtra={
        !editing ? (
          <Button variant="outline" size="sm" onClick={startEdit} className="h-8 text-xs font-medium">
            {t("settings.edit", "Edit")}
          </Button>
        ) : (
          <Button variant="outline" size="sm" onClick={cancelEdit} className="h-8 text-xs">
            {t("settings.cancel", "Cancel")}
          </Button>
        )
      }
    >
      {!editing ? (
        <div className="rounded-lg border border-border/70 bg-card p-3.5">
          <div className="text-xs font-medium text-muted-foreground">
            {t("settings.consoleKey", "Console API key")}
          </div>
          <div className="mt-2 font-mono text-sm font-semibold text-foreground">
            {stored
              ? maskKey(stored)
              : t("settings.keyNotSet", "Not set")}
          </div>
        </div>
      ) : (
        <div className="flex flex-col gap-3">
          <p className="text-sm text-muted-foreground">
            {t(
              "settings.consoleKeyHelp",
              "Paste the plaintext key shown once at create/rotate. It stays in this browser tab only.",
            )}
          </p>
          <div className="flex flex-col gap-2">
            <Label htmlFor="console-api-key">{t("settings.consoleKey", "Console API key")}</Label>
            <Input
              id="console-api-key"
              type="password"
              autoComplete="off"
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
            />
          </div>
          <SettingsSaveBar
            saving={saving}
            message={saveMessage}
            onReset={cancelEdit}
            onSave={save}
          />
        </div>
      )}
    </SectionCard>
  )
}
