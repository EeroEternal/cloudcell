function resolveApiBase(): string {
  const fromEnv = (import.meta.env.VITE_API_BASE as string | undefined)?.replace(/\/$/, "")
  if (fromEnv) return fromEnv
  if (typeof window !== "undefined") {
    const host = window.location.hostname
    if (host === "cloudcell.dev" || host === "www.cloudcell.dev") {
      return "https://api.cloudcell.dev"
    }
  }
  return ""
}

const API_BASE = resolveApiBase()
const API_KEY_STORAGE = "cloudcell.api_key"
const SESSION_STORAGE = "cloudcell.session"

export class ApiError extends Error {
  status: number

  constructor(status: number, message: string) {
    super(message)
    this.name = "ApiError"
    this.status = status
    Object.setPrototypeOf(this, new.target.prototype)
  }
}

function isApiError(err: unknown): err is ApiError {
  return (
    typeof err === "object" &&
    err !== null &&
    (err as ApiError).name === "ApiError" &&
    typeof (err as ApiError).status === "number"
  )
}

export function getApiKey(): string {
  return sessionStorage.getItem(API_KEY_STORAGE) ?? ""
}

export function setApiKey(key: string | null): void {
  if (key) sessionStorage.setItem(API_KEY_STORAGE, key)
  else sessionStorage.removeItem(API_KEY_STORAGE)
}

export function getSession(): string {
  return sessionStorage.getItem(SESSION_STORAGE) ?? ""
}

export function setSession(token: string | null): void {
  if (token) sessionStorage.setItem(SESSION_STORAGE, token)
  else sessionStorage.removeItem(SESSION_STORAGE)
}

export function authToken(): string {
  return getSession() || getApiKey()
}

export type AuthStatus = {
  registration_enabled: boolean
  has_users: boolean
  authenticated: boolean
  email: string | null
}

export type AuthResponse = {
  token: string
  email: string
}

export function isConflict(err: unknown): boolean {
  return isApiError(err) && err.status === 409
}

export function errorMessage(err: unknown, fallback: string): string {
  if (typeof err === "object" && err !== null && "message" in err) {
    const message = (err as { message: unknown }).message
    if (typeof message === "string" && message.trim()) return message
  }
  return fallback
}

export async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers)
  if (init?.body && !headers.has("content-type")) {
    headers.set("content-type", "application/json")
  }
  const token = authToken()
  if (token && !headers.has("authorization")) {
    headers.set("authorization", `Bearer ${token}`)
  }
  const response = await fetch(`${API_BASE}${path}`, { ...init, headers })
  const text = await response.text()
  let data: { error?: { message?: string } } = {}
  if (text) {
    try {
      data = JSON.parse(text) as { error?: { message?: string } }
    } catch {
      data = {}
    }
  }
  if (!response.ok) {
    throw new ApiError(
      response.status,
      data.error?.message || response.statusText || "request failed",
    )
  }
  return data as T
}

export type Sandbox = {
  id: string
  snapshot: string
  state: "pending" | "running" | "stopped"
  cpu: number
  mem_bytes: number
  pids: number
  egress: string[]
  created_at: string
}

export type Snapshot = {
  id: string
  name: string
  language: string
  status: string
}

export type ApiKeyListItem = {
  id: string
  name: string
  prefix: string
  hint: string
  created_at: string
}

export type ApiKeyCreated = ApiKeyListItem & {
  key: string
}
