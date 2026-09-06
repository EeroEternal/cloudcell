CREATE TABLE email_codes (
    email TEXT PRIMARY KEY NOT NULL,
    code_hash TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    verified INTEGER NOT NULL DEFAULT 0
);
