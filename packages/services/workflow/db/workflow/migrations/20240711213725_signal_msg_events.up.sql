-- Stores sent signals for replay
CREATE TABLE workflow_signal_send_events (
  workflow_id UUID NOT NULL REFERENCES workflows,
  location INT[] NOT NULL,
  signal_id UUID NOT NULL,
  signal_name TEXT NOT NULL,
  body JSONB NOT NULL,

  PRIMARY KEY (workflow_id, location)
);

-- Stores messages signals for replay
CREATE TABLE workflow_message_send_events (
  workflow_id UUID NOT NULL REFERENCES workflows,
  location INT[] NOT NULL,
  tags JSONB NOT NULL,
  message_name TEXT NOT NULL,
  body JSONB NOT NULL,

  PRIMARY KEY (workflow_id, location)
);
