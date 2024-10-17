-- NOTE: The indexes and constraints here could be improved if CRDB implements indexes for JSONB
-- https://go.crdb.dev/issue-v/35730/v23.1
-- https://go.crdb.dev/issue-v/48026/v23.1

-- Activities
ALTER TABLE workflow_activity_events
	ADD COLUMN version INT NOT NULL DEFAULT 1,
	-- Deprecated
	ALTER COLUMN location SET DEFAULT '{}',
	ALTER COLUMN loop_location SET DEFAULT '{}',
	-- New
	ADD COLUMN location2 JSONB,
	ADD COLUMN loop_location2 JSONB,
	-- Added solely because we can't delete or replace the current pkey. See issues at top of file.
	ADD COLUMN _uuid UUID NOT NULL DEFAULT gen_random_uuid(),
	-- Required to create indexes (JSONB indexes are not supported)
	ADD COLUMN location2_hash BYTES GENERATED ALWAYS AS (digest(location2::TEXT, 'md5')) STORED,
	ADD CONSTRAINT workflow_activity_events_location_unique UNIQUE (workflow_id, location2),
	ADD COLUMN loop_location2_hash BYTES GENERATED ALWAYS AS (digest(loop_location2::TEXT, 'md5')) STORED;

-- Update idx
DROP INDEX workflow_activity_events@idx_workflow_activity_events_loop_location2;
CREATE INDEX ON workflow_activity_events (workflow_id, loop_location2_hash)
WHERE forgotten = FALSE;

-- Signals
ALTER TABLE workflow_signal_events
	ADD COLUMN version INT NOT NULL DEFAULT 1,
	-- Deprecated
	ALTER COLUMN location SET DEFAULT '{}',
	ALTER COLUMN loop_location SET DEFAULT '{}',
	-- New
	ADD COLUMN location2 JSONB,
	ADD COLUMN loop_location2 JSONB,
	-- Added solely because we can't delete or replace the current pkey. See issues at top of file.
	ADD COLUMN _uuid UUID NOT NULL DEFAULT gen_random_uuid(),
	-- Required to create indexes (JSONB indexes are not supported)
	ADD COLUMN location2_hash BYTES GENERATED ALWAYS AS (digest(location2::TEXT, 'md5')) STORED,
	ADD CONSTRAINT workflow_signal_events_location_unique UNIQUE (workflow_id, location2),
	ADD COLUMN loop_location2_hash BYTES GENERATED ALWAYS AS (digest(loop_location2::TEXT, 'md5')) STORED;

-- Update idx
DROP INDEX workflow_signal_events@idx_workflow_signal_events_loop_location2;
CREATE INDEX ON workflow_signal_events (workflow_id, loop_location2_hash)
WHERE forgotten = FALSE;

-- Signal send
ALTER TABLE workflow_signal_send_events
	ADD COLUMN version INT NOT NULL DEFAULT 1,
	-- Deprecated
	ALTER COLUMN location SET DEFAULT '{}',
	ALTER COLUMN loop_location SET DEFAULT '{}',
	-- New
	ADD COLUMN location2 JSONB,
	ADD COLUMN loop_location2 JSONB,
	-- Added solely because we can't delete or replace the current pkey. See issues at top of file.
	ADD COLUMN _uuid UUID NOT NULL DEFAULT gen_random_uuid(),
	-- Required to create indexes (JSONB indexes are not supported)
	ADD COLUMN location2_hash BYTES GENERATED ALWAYS AS (digest(location2::TEXT, 'md5')) STORED,
	ADD CONSTRAINT workflow_signal_send_events_location_unique UNIQUE (workflow_id, location2),
	ADD COLUMN loop_location2_hash BYTES GENERATED ALWAYS AS (digest(loop_location2::TEXT, 'md5')) STORED;

-- Update idx
DROP INDEX workflow_signal_send_events@idx_workflow_signal_send_events_loop_location2;
CREATE INDEX ON workflow_signal_send_events (workflow_id, loop_location2_hash)
WHERE forgotten = FALSE;

-- Message send
ALTER TABLE workflow_message_send_events
	ADD COLUMN version INT NOT NULL DEFAULT 1,
	-- Deprecated
	ALTER COLUMN location SET DEFAULT '{}',
	ALTER COLUMN loop_location SET DEFAULT '{}',
	-- New
	ADD COLUMN location2 JSONB,
	ADD COLUMN loop_location2 JSONB,
	-- Added solely because we can't delete or replace the current pkey. See issues at top of file.
	ADD COLUMN _uuid UUID NOT NULL DEFAULT gen_random_uuid(),
	-- Required to create indexes (JSONB indexes are not supported)
	ADD COLUMN location2_hash BYTES GENERATED ALWAYS AS (digest(location2::TEXT, 'md5')) STORED,
	ADD CONSTRAINT workflow_message_send_events_location_unique UNIQUE (workflow_id, location2),
	ADD COLUMN loop_location2_hash BYTES GENERATED ALWAYS AS (digest(loop_location2::TEXT, 'md5')) STORED;

-- Update idx
DROP INDEX workflow_message_send_events@idx_workflow_message_send_events_loop_location2;
CREATE INDEX ON workflow_message_send_events (workflow_id, loop_location2_hash)
WHERE forgotten = FALSE;

-- Sub workflows
ALTER TABLE workflow_sub_workflow_events
	ADD COLUMN version INT NOT NULL DEFAULT 1,
	-- Deprecated
	ALTER COLUMN location SET DEFAULT '{}',
	ALTER COLUMN loop_location SET DEFAULT '{}',
	-- New
	ADD COLUMN location2 JSONB,
	ADD COLUMN loop_location2 JSONB,
	-- Added solely because we can't delete or replace the current pkey. See issues at top of file.
	ADD COLUMN _uuid UUID NOT NULL DEFAULT gen_random_uuid(),
	-- Required to create indexes (JSONB indexes are not supported)
	ADD COLUMN location2_hash BYTES GENERATED ALWAYS AS (digest(location2::TEXT, 'md5')) STORED,
	ADD CONSTRAINT workflow_sub_workflow_events_location_unique UNIQUE (workflow_id, location2),
	ADD COLUMN loop_location2_hash BYTES GENERATED ALWAYS AS (digest(loop_location2::TEXT, 'md5')) STORED;

-- Update idx
DROP INDEX workflow_sub_workflow_events@idx_workflow_sub_workflow_events_loop_location2;
CREATE INDEX ON workflow_sub_workflow_events (workflow_id, loop_location2_hash)
WHERE forgotten = FALSE;

-- Loops
ALTER TABLE workflow_loop_events
	ADD COLUMN version INT NOT NULL DEFAULT 1,
	-- Deprecated
	ALTER COLUMN location SET DEFAULT '{}',
	ALTER COLUMN loop_location SET DEFAULT '{}',
	-- New
	ADD COLUMN location2 JSONB,
	ADD COLUMN loop_location2 JSONB,
	-- Added solely because we can't delete or replace the current pkey. See issues at top of file.
	ADD COLUMN _uuid UUID NOT NULL DEFAULT gen_random_uuid(),
	-- Required to create indexes (JSONB indexes are not supported)
	ADD COLUMN location2_hash BYTES GENERATED ALWAYS AS (digest(location2::TEXT, 'md5')) STORED,
	ADD CONSTRAINT workflow_loop_events_location_unique UNIQUE (workflow_id, location2),
	ADD COLUMN loop_location2_hash BYTES GENERATED ALWAYS AS (digest(loop_location2::TEXT, 'md5')) STORED;

-- Update idx
DROP INDEX workflow_loop_events@idx_workflow_loop_events_loop_location2;
CREATE INDEX ON workflow_loop_events (workflow_id, loop_location2_hash)
WHERE forgotten = FALSE;

-- Sleep
ALTER TABLE workflow_sleep_events
	ADD COLUMN version INT NOT NULL DEFAULT 1,
	-- Deprecated
	ALTER COLUMN location SET DEFAULT '{}',
	ALTER COLUMN loop_location SET DEFAULT '{}',
	-- New
	ADD COLUMN location2 JSONB,
	ADD COLUMN loop_location2 JSONB,
	-- Added solely because we can't delete or replace the current pkey. See issues at top of file.
	ADD COLUMN _uuid UUID NOT NULL DEFAULT gen_random_uuid(),
	-- Required to create indexes (JSONB indexes are not supported)
	ADD COLUMN location2_hash BYTES GENERATED ALWAYS AS (digest(location2::TEXT, 'md5')) STORED,
	ADD CONSTRAINT workflow_sleep_events_location_unique UNIQUE (workflow_id, location2),
	ADD COLUMN loop_location2_hash BYTES GENERATED ALWAYS AS (digest(loop_location2::TEXT, 'md5')) STORED;

-- Update idx
DROP INDEX workflow_sleep_events@idx_workflow_sleep_events_loop_location;
CREATE INDEX ON workflow_sleep_events (workflow_id, loop_location2_hash)
WHERE forgotten = FALSE;

ALTER TABLE workflow_activity_errors
	-- Deprecated
	ALTER COLUMN location SET DEFAULT '{}',
	-- New
	ADD COLUMN location2 JSONB,
	-- Required to create indexes (JSONB indexes are not supported)
	ADD COLUMN location2_hash BYTES NOT NULL GENERATED ALWAYS AS (digest(location2::TEXT, 'md5')) STORED;

-- Update idx
CREATE INDEX ON workflow_activity_errors (workflow_id, location2_hash);

-- Branches
CREATE TABLE workflow_branch_events (
	workflow_id UUID NOT NULL REFERENCES workflows,
	location JSONB NOT NULL,

	loop_location JSONB,
	forgotten BOOLEAN NOT NULL DEFAULT FALSE,

	-- Required to create indexes (JSONB indexes are not supported)
	location_hash BYTES NOT NULL GENERATED ALWAYS AS (digest(location::TEXT, 'md5')) STORED,
	loop_location_hash BYTES GENERATED ALWAYS AS (digest(loop_location::TEXT, 'md5')) STORED,

	PRIMARY KEY (workflow_id, location_hash)
);

CREATE INDEX ON workflow_branch_events (workflow_id, loop_location_hash)
WHERE forgotten = FALSE;

-- Removed
CREATE TABLE workflow_removed_events (
	workflow_id UUID NOT NULL REFERENCES workflows,
	location JSONB NOT NULL,

	event_type INT NOT NULL, -- event::EventType
	event_name TEXT,

	loop_location JSONB,
	forgotten BOOLEAN NOT NULL DEFAULT FALSE,

	-- Required to create indexes (JSONB indexes are not supported)
	location_hash BYTES NOT NULL GENERATED ALWAYS AS (digest(location::TEXT, 'md5')) STORED,
	loop_location_hash BYTES GENERATED ALWAYS AS (digest(loop_location::TEXT, 'md5')) STORED,

	PRIMARY KEY (workflow_id, location_hash)
);

CREATE INDEX ON workflow_removed_events (workflow_id, loop_location_hash)
WHERE forgotten = FALSE;

-- Version checks
CREATE TABLE workflow_version_check_events (
	workflow_id UUID NOT NULL REFERENCES workflows,
	location JSONB NOT NULL,

	loop_location JSONB,
	forgotten BOOLEAN NOT NULL DEFAULT FALSE,

	-- Required to create indexes (JSONB indexes are not supported)
	location_hash BYTES NOT NULL GENERATED ALWAYS AS (digest(location::TEXT, 'md5')) STORED,
	loop_location_hash BYTES GENERATED ALWAYS AS (digest(loop_location::TEXT, 'md5')) STORED,

	PRIMARY KEY (workflow_id, location_hash)
);

CREATE INDEX ON workflow_version_check_events (workflow_id, loop_location_hash)
WHERE forgotten = FALSE;

