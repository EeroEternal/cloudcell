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
import { Input } from "@/components/ui/input"
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
import { api, setApiKey, type ApiKeyCreated, type ApiKeyListItem } from "@/lib/api"
import { t } from "@/lib/i18n"

export default function KeysPage() {
  const [rows, setRows] = useState<ApiKeyListItem[]>([])
  const [query, setQuery] = useState("")
  const [sort, setSort] = useState("new")
  const [selected, setSelected] = useState<string | null>(null)
  const [createOpen, setCreateOpen] = useState(false)
  const [name, setName] = useState("")
  const [saving, setSaving] = useState(false)
  const [revealed, setRevealed] = useState<ApiKeyCreated | null>(null)
  const [deleteId, setDeleteId] = useState<string | null>(null)
  const [rotateId, setRotateId] = useState<string | null>(null)

  const load = useCallback(async () => {
    setRows(await api<ApiKeyListItem[]>("/api/v1/keys"))
  }, [])

  useEffect(() => {
    let cancelled = false
    api<ApiKeyListItem[]>("/api/v1/keys")
      .then((items) => {
        if (!cancelled) setRows(items)
      })
      .catch((err: Error) => {
        if (!cancelled) toast.error(err.message)
      })
    return () => {
      cancelled = true
    }
  }, [])

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase()
    const next = rows.filter(
      (row) => !q || row.name.toLowerCase().includes(q) || row.prefix.toLowerCase().includes(q),
    )
    next.sort((a, b) => {
      const cmp = a.created_at.localeCompare(b.created_at)
      return sort === "new" ? -cmp : cmp
    })
    return next
  }, [query, rows, sort])

  const handleCreate = async () => {
    setSaving(true)
    try {
      const created = await api<ApiKeyCreated>("/api/v1/keys", {
        method: "POST",
        body: JSON.stringify({ name }),
      })
      setCreateOpen(false)
      setName("")
      setApiKey(created.key)
      setRevealed(created)
      await load()
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "create failed")
    } finally {
      setSaving(false)
    }
  }

  const handleRotate = async () => {
    if (!rotateId) return
    try {
      const created = await api<ApiKeyCreated>(`/api/v1/keys/${rotateId}/rotate`, {
        method: "POST",
      })
      setRotateId(null)
      setApiKey(created.key)
      setRevealed(created)
      await load()
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "rotate failed")
    }
  }

  const handleDelete = async () => {
    if (!deleteId) return
    try {
      await api(`/api/v1/keys/${deleteId}`, { method: "DELETE" })
      setDeleteId(null)
      toast.success(t("keys.deleted", "API key deleted"))
      await load()
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "delete failed")
    }
  }

  const copyKey = async () => {
    if (!revealed) return
    await navigator.clipboard.writeText(revealed.key)
    toast.success(t("keys.copied", "Key copied"))
  }

  const downloadKey = () => {
    if (!revealed) return
    const blob = new Blob([revealed.key], { type: "text/plain" })
    const url = URL.createObjectURL(blob)
    const link = document.createElement("a")
    link.href = url
    link.download = `api-key-${revealed.name}.txt`
    link.click()
    URL.revokeObjectURL(url)
  }

  const closeReveal = () => setRevealed(null)

  return (
    <PageShell>
      <PageContainer>
        <PageHeader
          title={t("keys.title", "API Keys")}
          action={
            <Button
              onClick={() => {
                setName("")
                setCreateOpen(true)
              }}
            >
              {t("keys.create", "Create")}
            </Button>
          }
        />
        <Card className="gap-0 p-4 sm:p-6">
          <EntityListToolbar
            searchValue={query}
            onSearchChange={setQuery}
            searchPlaceholder={t("keys.search", "Search name, prefix")}
            sort={
              <Select
                value={sort}
                onChange={setSort}
                className="w-52"
                options={[
                  { value: "new", label: t("keys.sortNew", "Created (New → Old)") },
                  { value: "old", label: t("keys.sortOld", "Created (Old → New)") },
                ]}
              />
            }
            resultCount={t("keys.count", `${filtered.length} items`)}
          />
          <Table className="table-fixed">
            <TableHeader className="sticky top-0 bg-card">
              <TableRow>
                <TableHead>{t("keys.colName", "Name")}</TableHead>
                <TableHead>{t("keys.colHint", "Key")}</TableHead>
                <TableHead>{t("keys.colId", "ID")}</TableHead>
                <TableHead className="w-40" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {filtered.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={4} className="text-muted-foreground">
                    {t("keys.empty", "No API keys")}
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
                    <TableCell>{row.name}</TableCell>
                    <TableCell className="font-mono text-xs">{row.hint}</TableCell>
                    <TableCell className="truncate font-mono text-xs">{row.id}</TableCell>
                    <TableCell>
                      <div className="flex items-center gap-1">
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={(event) => {
                            event.stopPropagation()
                            setRotateId(row.id)
                          }}
                        >
                          {t("keys.rotate", "Rotate")}
                        </Button>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={(event) => {
                            event.stopPropagation()
                            setDeleteId(row.id)
                          }}
                        >
                          {t("keys.delete", "Delete")}
                        </Button>
                      </div>
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
            <DialogTitle>{t("keys.createTitle", "Create API key")}</DialogTitle>
          </DialogHeader>
          <div className="flex flex-col gap-2">
            <Label htmlFor="key-name">{t("keys.name", "Name")}</Label>
            <Input
              id="key-name"
              value={name}
              onChange={(event) => setName(event.target.value)}
            />
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setCreateOpen(false)}>
              {t("common.cancel", "Cancel")}
            </Button>
            <Button onClick={handleCreate} disabled={saving || !name.trim()}>
              {t("keys.create", "Create")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog open={revealed !== null} onOpenChange={(open) => !open && closeReveal()}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>{t("keys.revealTitle", "API key created")}</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">
            {t(
              "keys.revealWarn",
              "This key is shown once. After you close this dialog the plaintext cannot be recovered. Store it now, or rotate later to mint a new key.",
            )}
          </p>
          <pre className="overflow-x-auto rounded-md bg-muted p-3 font-mono text-xs">
            {revealed?.key}
          </pre>
          <p className="font-mono text-xs text-muted-foreground">
            curl -H "Authorization: Bearer {revealed?.key}" https://api.cloudcell.dev/api/v1/sandboxes
          </p>
          <DialogFooter>
            <Button variant="outline" onClick={copyKey}>
              {t("keys.copy", "Copy key")}
            </Button>
            <Button variant="outline" onClick={downloadKey}>
              {t("keys.download", "Download .txt")}
            </Button>
            <Button onClick={closeReveal}>{t("keys.done", "Done")}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <ConfirmAlertDialog
        open={deleteId !== null}
        onOpenChange={(open) => {
          if (!open) setDeleteId(null)
        }}
        title={t("keys.deleteTitle", "Delete API key")}
        description={t("keys.deleteBody", "Requests signed with this key will fail immediately.")}
        cancelText={t("common.cancel", "Cancel")}
        confirmText={t("keys.delete", "Delete")}
        onConfirm={handleDelete}
      />

      <ConfirmAlertDialog
        open={rotateId !== null}
        onOpenChange={(open) => {
          if (!open) setRotateId(null)
        }}
        title={t("keys.rotateTitle", "Rotate API key")}
        description={t(
          "keys.rotateBody",
          "The current key is invalidated immediately. A new plaintext is shown once.",
        )}
        cancelText={t("common.cancel", "Cancel")}
        confirmText={t("keys.rotate", "Rotate")}
        onConfirm={handleRotate}
      />
    </PageShell>
  )
}
