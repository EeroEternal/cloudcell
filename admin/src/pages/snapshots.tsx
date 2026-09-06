import { useEffect, useState } from "react"
import { toast } from "sonner"
import { DetailPanel } from "@/components/layout/detail-panel"
import { PageContainer } from "@/components/layout/page-container"
import { PageHeader } from "@/components/layout/page-header"
import { PageShell } from "@/components/layout/page-shell"
import { TwoPanelLayout } from "@/components/layout/two-panel-layout"
import { Badge } from "@/components/ui/badge"
import { Card } from "@/components/ui/card"
import { api, type Snapshot } from "@/lib/api"
import { t } from "@/lib/i18n"

export default function SnapshotsPage() {
  const [items, setItems] = useState<Snapshot[]>([])
  const [selectedId, setSelectedId] = useState<string | null>(null)

  useEffect(() => {
    api<Snapshot[]>("/api/v1/snapshots")
      .then((snaps) => {
        setItems(snaps)
        setSelectedId((current) => current ?? snaps[0]?.id ?? null)
      })
      .catch((err: Error) => toast.error(err.message))
  }, [])

  const selected = items.find((item) => item.id === selectedId) ?? items[0]

  return (
    <PageShell>
      <PageContainer>
        <PageHeader title={t("snapshots.title", "Snapshots")} />
        <TwoPanelLayout
          workspace
          left={
            <Card className="min-h-0 flex-1 gap-0 overflow-hidden p-0">
              <ul className="h-full overflow-y-auto p-2">
                {items.map((item) => (
                  <li key={item.id}>
                    <button
                      type="button"
                      onClick={() => setSelectedId(item.id)}
                      className={`flex w-full rounded-md px-3 py-2 text-left text-sm leading-5 ${
                        item.id === selectedId
                          ? "bg-primary/10 font-medium text-primary"
                          : "hover:bg-muted"
                      }`}
                    >
                      {item.name}
                    </button>
                  </li>
                ))}
              </ul>
            </Card>
          }
          right={
            <DetailPanel>
              {selected ? (
                <>
                  <h2 className="text-section-title">{selected.name}</h2>
                  <div className="mt-3 flex flex-col gap-2">
                    <Badge variant="warning">{selected.status}</Badge>
                    <p className="text-meta-sm text-muted-foreground">
                      {t("snapshots.language", "Language")}: {selected.language}
                    </p>
                    <p className="text-meta-sm text-muted-foreground">
                      {t(
                        "snapshots.declared",
                        "Declared on the control plane. Packing an erofs rootfs onto a cell node is not implemented yet.",
                      )}
                    </p>
                  </div>
                </>
              ) : (
                <p className="text-meta-sm text-muted-foreground">
                  {t("snapshots.empty", "No snapshots")}
                </p>
              )}
            </DetailPanel>
          }
        />
      </PageContainer>
    </PageShell>
  )
}
