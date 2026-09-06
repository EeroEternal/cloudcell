import { useCallback, useEffect, useMemo, useState } from "react"
import { toast } from "sonner"
import { PageContainer } from "@/components/layout/page-container"
import { PageHeader } from "@/components/layout/page-header"
import { PageShell } from "@/components/layout/page-shell"
import { EntityListToolbar } from "@/components/entity-list/EntityListToolbar"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { ConfirmAlertDialog } from "@/components/ui/confirm-alert-dialog"
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Label } from "@/components/ui/label"
import { Select } from "@/components/ui/select"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { Badge } from "@/components/ui/badge"
import { api, type Sandbox, type Snapshot } from "@/lib/api"
import { t } from "@/lib/i18n"

function formatMem(bytes: number): string {
  if (bytes >= 1 << 30) return `${(bytes / (1 << 30)).toFixed(0)}G`
  if (bytes >= 1 << 20) return `${(bytes / (1 << 20)).toFixed(0)}M`
  return `${bytes}B`
}

function stateVariant(state: Sandbox["state"]) {
  if (state === "running") return "success" as const
  if (state === "pending") return "warning" as const
  return "inactive" as const
}

export default function SandboxesPage() {
  const [rows, setRows] = useState<Sandbox[]>([])
  const [snapshots, setSnapshots] = useState<Snapshot[]>([])
  const [query, setQuery] = useState("")
  const [status, setStatus] = useState("all")
  const [sort, setSort] = useState("new")
  const [selected, setSelected] = useState<string | null>(null)
  const [createOpen, setCreateOpen] = useState(false)
  const [snapshotId, setSnapshotId] = useState("base")
  const [creating, setCreating] = useState(false)
  const [deleteId, setDeleteId] = useState<string | null>(null)

  const load = useCallback(async () => {
    const [sbx, snaps] = await Promise.all([
      api<Sandbox[]>("/api/v1/sandboxes"),
      api<Snapshot[]>("/api/v1/snapshots"),
    ])
    setRows(sbx)
    setSnapshots(snaps)
    setSnapshotId((current) =>
      snaps.some((s) => s.id === current) ? current : snaps[0]?.id ?? "base",
    )
  }, [])

  useEffect(() => {
    let cancelled = false
    Promise.all([
      api<Sandbox[]>("/api/v1/sandboxes"),
      api<Snapshot[]>("/api/v1/snapshots"),
    ])
      .then(([sbx, snaps]) => {
        if (cancelled) return
        setRows(sbx)
        setSnapshots(snaps)
        setSnapshotId((current) =>
          snaps.some((s) => s.id === current) ? current : snaps[0]?.id ?? "base",
        )
      })
      .catch((err: Error) => {
        if (!cancelled) toast.error(err.message)
      })
    return () => {
      cancelled = true
    }
  }, [])

  const filtered = useMemo(() => {
    const next = rows.filter((row) => {
      const q = query.trim().toLowerCase()
      const matchesQuery =
        !q || row.id.toLowerCase().includes(q) || row.snapshot.toLowerCase().includes(q)
      const matchesStatus = status === "all" || row.state === status
      return matchesQuery && matchesStatus
    })
    next.sort((a, b) => {
      const cmp = a.created_at.localeCompare(b.created_at)
      return sort === "new" ? -cmp : cmp
    })
    return next
  }, [query, rows, sort, status])

  const handleCreate = async () => {
    setCreating(true)
    try {
      await api<Sandbox>("/api/v1/sandboxes", {
        method: "POST",
        body: JSON.stringify({ snapshot: snapshotId }),
      })
      setCreateOpen(false)
      toast.success(t("sandboxes.created", "Sandbox recorded"))
      await load()
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "create failed")
    } finally {
      setCreating(false)
    }
  }

  const handleDelete = async () => {
    if (!deleteId) return
    try {
      await api(`/api/v1/sandboxes/${deleteId}`, { method: "DELETE" })
      toast.success(t("sandboxes.deleted", "Sandbox deleted"))
      setDeleteId(null)
      await load()
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "delete failed")
    }
  }

  return (
    <PageShell>
      <PageContainer>
        <PageHeader
          title={t("sandboxes.title", "Sandboxes")}
          action={
            <Button onClick={() => setCreateOpen(true)}>
              {t("sandboxes.create", "Create")}
            </Button>
          }
        />
        <Card className="gap-0 p-4 sm:p-6">
          <EntityListToolbar
            searchValue={query}
            onSearchChange={setQuery}
            searchPlaceholder={t("sandboxes.search", "Search snapshot, id")}
            filters={
              <Select
                value={status}
                onChange={setStatus}
                className="w-40"
                options={[
                  { value: "all", label: t("sandboxes.allStates", "All states") },
                  { value: "pending", label: t("sandboxes.pending", "Pending") },
                  { value: "running", label: t("sandboxes.running", "Running") },
                  { value: "stopped", label: t("sandboxes.stopped", "Stopped") },
                ]}
              />
            }
            sort={
              <Select
                value={sort}
                onChange={setSort}
                className="w-52"
                options={[
                  { value: "new", label: t("sandboxes.sortNew", "Created (New → Old)") },
                  { value: "old", label: t("sandboxes.sortOld", "Created (Old → New)") },
                ]}
              />
            }
            resultCount={t("sandboxes.count", `${filtered.length} items`)}
          />
          <Table className="table-fixed">
            <TableHeader className="sticky top-0 bg-card">
              <TableRow>
                <TableHead>{t("sandboxes.colSnapshot", "Snapshot")}</TableHead>
                <TableHead>{t("sandboxes.colState", "State")}</TableHead>
                <TableHead>{t("sandboxes.colResources", "CPU / mem / pids")}</TableHead>
                <TableHead>{t("sandboxes.colId", "ID")}</TableHead>
                <TableHead className="w-24" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {filtered.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={5} className="text-muted-foreground">
                    {t("sandboxes.empty", "No sandboxes")}
                  </TableCell>
                </TableRow>
              ) : (
                filtered.map((row) => (
                  <TableRow
                    key={row.id}
                    className={
                      selected === row.id ? "bg-primary/10 font-medium" : "hover:bg-muted/50"
                    }
                    onClick={() => setSelected(row.id)}
                  >
                    <TableCell>{row.snapshot}</TableCell>
                    <TableCell>
                      <Badge variant={stateVariant(row.state)}>{row.state}</Badge>
                    </TableCell>
                    <TableCell className="font-mono text-xs">
                      {row.cpu} / {formatMem(row.mem_bytes)} / {row.pids}
                    </TableCell>
                    <TableCell className="truncate font-mono text-xs">{row.id}</TableCell>
                    <TableCell>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={(event) => {
                          event.stopPropagation()
                          setDeleteId(row.id)
                        }}
                      >
                        {t("sandboxes.delete", "Delete")}
                      </Button>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </Card>
      </PageContainer>

      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>{t("sandboxes.createTitle", "Create sandbox")}</DialogTitle>
          </DialogHeader>
          <div className="flex flex-col gap-2">
            <Label htmlFor="snapshot">{t("sandboxes.snapshot", "Snapshot")}</Label>
            <Select
              id="snapshot"
              value={snapshotId}
              onChange={setSnapshotId}
              options={snapshots.map((s) => ({ value: s.id, label: s.name }))}
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setCreateOpen(false)}>
              {t("common.cancel", "Cancel")}
            </Button>
            <Button onClick={handleCreate} disabled={creating || snapshots.length === 0}>
              {t("sandboxes.create", "Create")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <ConfirmAlertDialog
        open={deleteId !== null}
        onOpenChange={(open) => {
          if (!open) setDeleteId(null)
        }}
        title={t("sandboxes.deleteTitle", "Delete sandbox")}
        description={t(
          "sandboxes.deleteBody",
          "Removes the control-plane record. No cell process is running yet.",
        )}
        cancelText={t("common.cancel", "Cancel")}
        confirmText={t("sandboxes.delete", "Delete")}
        onConfirm={handleDelete}
      />
    </PageShell>
  )
}
