import { useEffect, useState } from "react"
import { toast } from "sonner"
import { PageContainer } from "@/components/layout/page-container"
import { PageHeader } from "@/components/layout/page-header"
import { PageShell } from "@/components/layout/page-shell"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { Select } from "@/components/ui/select"
import { Textarea } from "@/components/ui/textarea"
import { api, ApiError, type Sandbox } from "@/lib/api"
import { t } from "@/lib/i18n"

export default function PlaygroundPage() {
  const [sandboxes, setSandboxes] = useState<Sandbox[]>([])
  const [sandboxId, setSandboxId] = useState("")
  const [command, setCommand] = useState("echo hello")
  const [output, setOutput] = useState("")
  const [running, setRunning] = useState(false)

  const selected = sandboxes.find((row) => row.id === sandboxId)

  useEffect(() => {
    api<Sandbox[]>("/api/v1/sandboxes")
      .then((rows) => {
        const sorted = [...rows].sort(
          (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
        )
        setSandboxes(sorted)
        setSandboxId((current) => current || sorted[0]?.id || "")
      })
      .catch((err: Error) => toast.error(err.message))
  }, [])

  const run = async () => {
    if (!sandboxId) return
    setRunning(true)
    setOutput("")
    try {
      const result = await api<{ stdout: string; code: number }>(
        `/api/v1/sandboxes/${sandboxId}/exec`,
        {
          method: "POST",
          body: JSON.stringify({ argv: ["bash", "-c", command] }),
        },
      )
      const trailer = result.code === 0 ? "" : `\n[exit ${result.code}]`
      setOutput(`${result.stdout}${trailer}`)
    } catch (err) {
      const message =
        err instanceof ApiError || err instanceof Error ? err.message : t("common.execFailed")
      setOutput(message)
    } finally {
      setRunning(false)
    }
  }

  return (
    <PageShell className="overflow-hidden">
      <PageContainer>
        <PageHeader title={t("playground.title", "Playground")} />
        <div className="flex min-h-0 flex-1 gap-4">
          <Card className="w-64 shrink-0 gap-0 overflow-y-auto p-3">
            <p className="text-label-sm text-muted-foreground">
              {t("playground.sandbox", "Sandbox")}
            </p>
            <div className="mt-2">
              <Select
                value={sandboxId}
                onChange={setSandboxId}
                options={sandboxes.map((row) => ({
                  value: row.id,
                  label: `${row.snapshot} · ${row.id.slice(0, 6)}`,
                }))}
                emptyText={t("playground.noSandbox", "Create a sandbox first")}
              />
            </div>
            {selected && (
              <p className="mt-2 text-label-sm text-muted-foreground">
                {t(`sandboxes.${selected.state}`, selected.state)} ·{" "}
                {new Date(selected.created_at).toLocaleString()}
              </p>
            )}
          </Card>
          <Card className="flex min-h-0 min-w-0 flex-1 flex-col gap-3 p-4">
            <Textarea
              value={command}
              onChange={(event) => setCommand(event.target.value)}
              className="min-h-24 font-mono text-sm"
              aria-label={t("playground.command", "Command")}
            />
            <div>
              <Button onClick={run} disabled={running || !sandboxId}>
                {t("playground.run", "Run")}
              </Button>
            </div>
            <pre className="min-h-0 flex-1 overflow-auto rounded-md bg-muted p-3 font-mono text-xs whitespace-pre-wrap">
              {output ||
                t(
                  "playground.hint",
                  "Run a command. Needs a running cell (CLOUDCELL_SAND) or returns 501.",
                )}
            </pre>
          </Card>
        </div>
      </PageContainer>
    </PageShell>
  )
}
