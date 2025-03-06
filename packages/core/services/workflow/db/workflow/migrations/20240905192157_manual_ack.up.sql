ALTER TABLE workflows
ADD COLUMN silence_ts INT;

ALTER TABLE signals
ADD COLUMN silence_ts INT;

ALTER TABLE tagged_signals
ADD COLUMN silence_ts INT;

CREATE INDEX ON signals (ack_ts, silence_ts) STORING (workflow_id, signal_name);
CREATE INDEX ON tagged_signals (ack_ts, silence_ts) STORING (tags, signal_name);
