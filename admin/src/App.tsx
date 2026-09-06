import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom"
import { DashboardLayout } from "@/components/layout/dashboard-layout"
import DashboardPage from "@/pages/dashboard"
import KeysPage from "@/pages/keys"
import PlaygroundPage from "@/pages/playground"
import SandboxesPage from "@/pages/sandboxes"
import SettingsPage from "@/pages/settings"
import SnapshotsPage from "@/pages/snapshots"

export default function App() {
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
