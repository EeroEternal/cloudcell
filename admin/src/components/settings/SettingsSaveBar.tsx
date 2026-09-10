import { PageActionBar } from "@/common/page-action-bar"
import { Button } from "@/components/ui/button"
import { Loader2, RefreshCw, Save } from "lucide-react"
import { t } from "@/lib/i18n"

interface SettingsSaveBarProps {
  saving: boolean
  message?: { type: "success" | "error"; text: string } | null
  onReset: () => void
  onSave: () => void
  resetLabel?: string
  saveLabel?: string
}

export function SettingsSaveBar({
  saving,
  message,
  onReset,
  onSave,
  resetLabel,
  saveLabel,
}: SettingsSaveBarProps) {
  return (
    <PageActionBar
      leading={
        <Button
          variant="outline"
          onClick={onReset}
          className="h-9 px-4"
        >
          <RefreshCw className="h-4 w-4 mr-2" />
          {resetLabel ?? t("common.cancel")}
        </Button>
      }
      trailing={
        <>
          {message ? (
            <div
              className={`rounded-lg px-3 py-1.5 text-xs font-medium ${
                message.type === "success"
                  ? "border border-success/30 bg-success/10 text-success"
                  : "border border-destructive/30 bg-destructive/10 text-destructive"
              }`}
            >
              {message.text}
            </div>
          ) : null}
          <Button
            onClick={onSave}
            disabled={saving}
            className="h-9 px-6"
          >
            {saving ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <Save className="mr-2 h-4 w-4" />}
            {saveLabel ?? t("common.save")}
          </Button>
        </>
      }
    />
  )
}
