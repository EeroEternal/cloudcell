const API_BASE = import.meta.env.VITE_API_BASE ?? ""

export class ApiError extends Error {
  status: number

  constructor(status: number, message: string) {
    super(message)
    this.status = status
  }
}

export async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers)
  if (init?.body && !headers.has("content-type")) {
    headers.set("content-type", "application/json")
  }
  const response = await fetch(`${API_BASE}${path}`, { ...init, headers })
  const text = await response.text()
  const data = text ? (JSON.parse(text) as { error?: { message?: string } } & T) : ({} as T)
  if (!response.ok) {
    const message =
      (data as { error?: { message?: string } }).error?.message ||
      response.statusText ||
      "request failed"
    throw new ApiError(response.status, message)
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
