ALTER TABLE sandboxes ADD COLUMN user_id TEXT REFERENCES users(id);
ALTER TABLE api_keys ADD COLUMN user_id TEXT REFERENCES users(id);

UPDATE sandboxes
SET user_id = (SELECT id FROM users ORDER BY created_at ASC LIMIT 1)
WHERE user_id IS NULL AND EXISTS (SELECT 1 FROM users);

UPDATE api_keys
SET user_id = (SELECT id FROM users ORDER BY created_at ASC LIMIT 1)
WHERE user_id IS NULL AND EXISTS (SELECT 1 FROM users);

DELETE FROM sandboxes WHERE user_id IS NULL;
DELETE FROM api_keys WHERE user_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_sandboxes_user ON sandboxes(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_user ON api_keys(user_id);
