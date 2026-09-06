import { useEffect, useState } from "react"
import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom"
import { DashboardLayout } from "@/components/layout/dashboard-layout"
import { PageLoading } from "@/components/ui/page-loading"
import { api, type AuthStatus } from "@/lib/api"
import AuthScreen from "@/pages/auth"
import DashboardPage from "@/pages/dashboard"
import KeysPage from "@/pages/keys"
import PlaygroundPage from "@/pages/playground"
import SandboxesPage from "@/pages/sandboxes"
import SettingsPage from "@/pages/settings"
import SnapshotsPage from "@/pages/snapshots"

export default function App() {
  const [status, setStatus] = useState<AuthStatus | null>(null)

  const refresh = () => {
    api<AuthStatus>("/api/v1/auth/status")
      .then(setStatus)
      .catch(() =>
        setStatus({
          registration_enabled: true,
          has_users: false,
          authenticated: false,
          email: null,
        }),
      )
  }

  useEffect(() => {
    refresh()
  }, [])

  if (!status) {
    return <PageLoading />
  }

  if (!status.authenticated) {
    return <AuthScreen status={status} onAuthenticated={refresh} />
  }

  return (
    <BrowserRouter>
      <Routes>
        <Route element={<DashboardLayout />}>
          <Route index element={<DashboardPage />} />
          <Route path="sandboxes" element={<SandboxesPage />} />
          <Route path="snapshots" element={<SnapshotsPage />} />
          <Route path="playground" element={<PlaygroundPage />} />
          <Route path="keys" element={<KeysPage />} />
          <Route path="settings" element={<SettingsPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </BrowserRouter>
  )
}
