import { Navigate, Outlet, Route, Routes } from "react-router-dom"
import { BrowserRouter } from "react-router-dom"
import { DashboardLayout } from "@/components/layout/dashboard-layout"
import { getSession } from "@/lib/api"
import DashboardPage from "@/pages/dashboard"
import KeysPage from "@/pages/keys"
import LoginPage from "@/pages/login"
import PlaygroundPage from "@/pages/playground"
import RegisterPage from "@/pages/register"
import SandboxesPage from "@/pages/sandboxes"
import SettingsPage from "@/pages/settings"
import SnapshotsPage from "@/pages/snapshots"
import HelpPage from "@/pages/help"

function RequireAuth() {
  if (!getSession()) return <Navigate to="/login" replace />
  return <DashboardLayout />
}

function AuthOnly() {
  if (getSession()) return <Navigate to="/" replace />
  return <Outlet />
}

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="help" element={<HelpPage />} />
        <Route element={<AuthOnly />}>
          <Route path="login" element={<LoginPage />} />
          <Route path="register" element={<RegisterPage />} />
        </Route>
        <Route element={<RequireAuth />}>
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
