-- Key/value store for application preferences (theme, density, window state,
-- master-password hash, ...). Values are serialized as JSON strings so we can
-- persist any small payload without schema churn.
CREATE TABLE IF NOT EXISTS settings (
    key         TEXT PRIMARY KEY NOT NULL,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
