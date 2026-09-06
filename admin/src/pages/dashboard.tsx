import { useEffect, useState } from "react"
import { Box, Clock, Layers, Server } from "lucide-react"
import {
  analyticsGridClassName,
  dashboardMonitoringCardClassName,
  dashboardMonitoringHeaderClassName,
} from "@/common/card-height-presets"
import { InsightList } from "@/common/insight-list"
import { InsightListItem } from "@/common/insight-list-item"
import { InsightPanel } from "@/common/insight-panel"
import { MetricTile } from "@/common/metric-tile"
import { PageContainer } from "@/components/layout/page-container"
import { PageHeader } from "@/components/layout/page-header"
import { PageShell } from "@/components/layout/page-shell"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { StatCard } from "@/components/ui/stat-card"
import { t } from "@/lib/i18n"
import { api, type Sandbox, type Snapshot } from "@/lib/api"

export default function DashboardPage() {
  const [sandboxes, setSandboxes] = useState<Sandbox[]>([])
  const [snapshots, setSnapshots] = useState<Snapshot[]>([])
  const [reachable, setReachable] = useState(false)

  useEffect(() => {
    let cancelled = false
    Promise.all([
      api<Sandbox[]>("/api/v1/sandboxes"),
      api<Snapshot[]>("/api/v1/snapshots"),
    ])
      .then(([sbx, snaps]) => {
        if (cancelled) return
        setSandboxes(sbx)
        setSnapshots(snaps)
        setReachable(true)
      })
      .catch(() => {
        if (!cancelled) setReachable(false)
      })
    return () => {
      cancelled = true
    }
  }, [])

  const pending = sandboxes.filter((s) => s.state === "pending").length

  return (
    <PageShell className="overflow-y-auto">
      <PageContainer>
        <PageHeader title={t("dashboard.title", "Dashboard")} />
        <div className="flex flex-col gap-6 pb-6">
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4">
            <StatCard
              title={t("dashboard.sandboxes", "Sandboxes")}
              value={String(sandboxes.length)}
              icon={Box}
            />
            <StatCard
              title={t("dashboard.snapshots", "Snapshots")}
              value={String(snapshots.length)}
              icon={Layers}
            />
            <StatCard
              title={t("dashboard.pending", "Pending cells")}
              value={String(pending)}
              icon={Clock}
            />
            <StatCard
              title={t("dashboard.api", "Control plane")}
              value={reachable ? t("dashboard.up", "Up") : t("dashboard.down", "Down")}
              icon={Server}
            />
          </div>

          <div className={analyticsGridClassName}>
            <InsightPanel title={t("dashboard.snapshotsPanel", "Snapshots")}>
              <InsightList>
                {snapshots.map((snap) => (
                  <InsightListItem key={snap.id}>
                    <div className="flex items-center justify-between gap-2">
                      <span className="text-sm font-medium">{snap.name}</span>
                      <span className="text-meta-sm text-muted-foreground">{snap.status}</span>
                    </div>
                  </InsightListItem>
                ))}
              </InsightList>
            </InsightPanel>
            <div className="min-h-0 xl:col-span-8">
              <Card className={dashboardMonitoringCardClassName}>
                <CardHeader className={dashboardMonitoringHeaderClassName}>
                  <CardTitle>{t("dashboard.runtime", "Runtime")}</CardTitle>
                </CardHeader>
                <CardContent className="grid min-h-0 flex-1 grid-cols-2 content-start gap-2 px-5 pb-5">
                  <MetricTile
                    label={t("dashboard.exec", "Exec")}
                    value={t("dashboard.execHint", "sand serve")}
                  />
                  <MetricTile
                    label={t("dashboard.agentcell", "AgentCell")}
                    value={t("dashboard.agentcellHint", "CLOUDCELL_SAND")}
                  />
                </CardContent>
              </Card>
            </div>
          </div>
        </div>
      </PageContainer>
    </PageShell>
  )
}
