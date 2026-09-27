ALTER TABLE sandboxes ADD COLUMN caches_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE sandboxes ADD COLUMN disk_bytes INTEGER;
