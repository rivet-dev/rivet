# Aggregate entire history

```sql
SELECT location, 'activity' AS t, activity_name AS name
FROM db_workflow.workflow_activity_events
WHERE
	workflow_id = $1 AND NOT forgotten
UNION ALL
SELECT location, 'signal' AS t, signal_name AS name
FROM db_workflow.workflow_signal_events
WHERE
	workflow_id = $1 AND NOT forgotten
UNION ALL
SELECT location, 'sub_workflow' AS t, sub_workflow_id::STRING AS name
FROM db_workflow.workflow_sub_workflow_events
WHERE
	workflow_id = $1 AND NOT forgotten
UNION ALL
SELECT location, 'signal_send' AS t, signal_name AS name
FROM db_workflow.workflow_signal_send_events
WHERE
	workflow_id = $1 AND NOT forgotten
UNION ALL
SELECT location, 'message_send' AS t, message_name AS name
FROM db_workflow.workflow_message_send_events
WHERE
	workflow_id = $1 AND NOT forgotten
UNION ALL
SELECT location, 'loop' AS t, NULL AS name
FROM db_workflow.workflow_loop_events
WHERE
	workflow_id = $1 AND NOT forgotten
ORDER BY location ASC;
```

> Remove `AND NOT forgotten` to show all events, including past loop executions
