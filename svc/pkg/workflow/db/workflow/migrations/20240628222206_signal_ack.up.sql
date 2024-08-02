-- Conditional index for selecting before ack'd
CREATE UNIQUE INDEX idx_signals_workflow_id
ON signals (workflow_id)
WHERE ack_ts IS NULL;
