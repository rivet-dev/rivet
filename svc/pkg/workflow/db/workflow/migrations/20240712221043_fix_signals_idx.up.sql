DROP INDEX IF EXISTS idx_signals_workflow_id;

CREATE INDEX idx_signals_workflow_id
ON signals (workflow_id)
WHERE ack_ts IS NULL;
