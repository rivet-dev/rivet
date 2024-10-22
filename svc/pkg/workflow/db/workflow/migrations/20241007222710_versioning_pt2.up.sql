-- Primary key changes (must be isolated)
-- https://go.crdb.dev/issue-v/45510/v23.1

ALTER TABLE workflow_activity_events
	DROP CONSTRAINT workflow_activity_events_pkey,
	ADD PRIMARY KEY (_uuid);

ALTER TABLE workflow_signal_events
	DROP CONSTRAINT workflow_signal_events_pkey,
	ADD PRIMARY KEY (_uuid);

ALTER TABLE workflow_signal_send_events
	DROP CONSTRAINT workflow_signal_send_events_pkey,
	ADD PRIMARY KEY (_uuid);

ALTER TABLE workflow_message_send_events
	DROP CONSTRAINT workflow_message_send_events_pkey,
	ADD PRIMARY KEY (_uuid);

ALTER TABLE workflow_sub_workflow_events
	DROP CONSTRAINT workflow_sub_workflow_events_pkey,
	ADD PRIMARY KEY (_uuid);

ALTER TABLE workflow_loop_events
	DROP CONSTRAINT workflow_loop_events_pkey,
	ADD PRIMARY KEY (_uuid);

ALTER TABLE workflow_sleep_events
	DROP CONSTRAINT workflow_sleep_events_pkey,
	ADD PRIMARY KEY (_uuid);


-- Backfill location2 for activities and loops (they cause problems with ON CONFLICT)
UPDATE db_workflow.workflow_activity_events AS a
SET location2 = (
	SELECT jsonb_agg(jsonb_build_array(x + 1))
	FROM unnest(a.location) AS x
)
FROM db_workflow.workflows AS w
WHERE
	a.workflow_id = w.workflow_id AND
	w.output IS NULL AND
	a.forgotten = false;

UPDATE workflow_loop_events
SET location2 = (
	SELECT jsonb_agg(jsonb_build_array(x + 1))
	FROM unnest(location) AS x
)
WHERE forgotten = FALSE;
