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
