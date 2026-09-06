CREATE TABLE api_keys (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    prefix TEXT NOT NULL,
    hash TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL
);

CREATE TABLE sandboxes (
    id TEXT PRIMARY KEY NOT NULL,
    snapshot TEXT NOT NULL,
    state TEXT NOT NULL,
    cpu REAL NOT NULL,
    mem_bytes INTEGER NOT NULL,
    pids INTEGER NOT NULL,
    egress_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);
