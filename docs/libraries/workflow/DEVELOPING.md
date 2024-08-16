# Developing

## View realtime logs

```logql
{name="monolith-workflow-worker"}
```

## Fixing errors

If you run in to a too many retries error on a workflow, then:

1. Update the workflow code
2. Re-wake the workflow
3. Wait for the workflow to poll for new changes

For a single workflow:

```sql
UPDATE db_workflow.workflows SET wake_immediate = true WHERE workflow_id = 'MY_ID':uuid;
```

For all workflows of a type:

```sql
UPDATE db_workflow.workflows SET wake_immediate = true WHERE workflow_name = 'MY_NAME';
```


# Visualize entire workflow history

```sql
WITH workflow_events AS (
    SELECT '1db61ba2-6271-40a5-9a38-e6fa212e6f7d'::uuid AS workflow_id
)
SELECT location, 'activity' AS t, activity_name, input, output, forgotten
FROM db_workflow.workflow_activity_events, workflow_events
WHERE
    workflow_activity_events.workflow_id = workflow_events.workflow_id
UNION ALL
SELECT location, 'signal' AS t, signal_name AS name, null as input, null as output, forgotten
FROM db_workflow.workflow_signal_events, workflow_events
WHERE
    workflow_signal_events.workflow_id = workflow_events.workflow_id
UNION ALL
SELECT location, 'sub_workflow' AS t, sub_workflow_id::STRING AS name, null as input, null as output, forgotten
FROM db_workflow.workflow_sub_workflow_events, workflow_events
WHERE
    workflow_sub_workflow_events.workflow_id = workflow_events.workflow_id
UNION ALL
SELECT location, 'signal_send' AS t, signal_name AS name, null as input, null as output, forgotten
FROM db_workflow.workflow_signal_send_events, workflow_events
WHERE
    workflow_signal_send_events.workflow_id = workflow_events.workflow_id
UNION ALL
SELECT location, 'message_send' AS t, message_name AS name, null as input, null as output, forgotten
FROM db_workflow.workflow_message_send_events, workflow_events
WHERE
    workflow_message_send_events.workflow_id = workflow_events.workflow_id
UNION ALL
SELECT location, 'loop' AS t, NULL AS name, null as input, null as output, forgotten
FROM db_workflow.workflow_loop_events, workflow_events
WHERE
    workflow_loop_events.workflow_id = workflow_events.workflow_id
ORDER BY location ASC;
```

## Completely delete workflow with ID

```sql
WITH workflow_ids AS (
    SELECT 'WORKFLOW_ID':uuid AS workflow_id
),
delete_activity_events AS (
    DELETE FROM db_workflow.workflow_activity_events
    WHERE workflow_id IN (SELECT workflow_id FROM workflow_ids)
    RETURNING 1
),
delete_signal_events AS (
    DELETE FROM db_workflow.workflow_signal_events
    WHERE workflow_id IN (SELECT workflow_id FROM workflow_ids)
    RETURNING 1
),
delete_sub_workflow_events AS (
    DELETE FROM db_workflow.workflow_sub_workflow_events
    WHERE workflow_id IN (SELECT workflow_id FROM workflow_ids)
    RETURNING 1
),
delete_signal_send_events AS (
    DELETE FROM db_workflow.workflow_signal_send_events
    WHERE workflow_id IN (SELECT workflow_id FROM workflow_ids)
    RETURNING 1
),
delete_message_send_events AS (
    DELETE FROM db_workflow.workflow_message_send_events
    WHERE workflow_id IN (SELECT workflow_id FROM workflow_ids)
    RETURNING 1
),
delete_loop_events AS (
    DELETE FROM db_workflow.workflow_loop_events
    WHERE workflow_id IN (SELECT workflow_id FROM workflow_ids)
    RETURNING 1
),
delete_activity_errors AS (
    DELETE FROM db_workflow.workflow_activity_errors
    WHERE workflow_id IN (SELECT workflow_id FROM workflow_ids)
    RETURNING 1
)
DELETE FROM db_workflow.workflows
WHERE workflow_id IN (SELECT workflow_id FROM workflow_ids)
RETURNING 1;
```

## Completely delete workflow with name

```sql
WITH workflow_ids AS (
    SELECT workflow_id
    FROM db_workflow.workflows
    WHERE workflow_name = 'WORKFLOW_NAME'
),
delete_activity_events AS (
    DELETE FROM db_workflow.workflow_activity_events
    WHERE workflow_id IN (SELECT workflow_id FROM workflow_ids)
    RETURNING 1
),
delete_signal_events AS (
    DELETE FROM db_workflow.workflow_signal_events
    WHERE workflow_id IN (SELECT workflow_id FROM workflow_ids)
    RETURNING 1
),
delete_sub_workflow_events AS (
    DELETE FROM db_workflow.workflow_sub_workflow_events
    WHERE workflow_id IN (SELECT workflow_id FROM workflow_ids)
    RETURNING 1
),
delete_signal_send_events AS (
    DELETE FROM db_workflow.workflow_signal_send_events
    WHERE workflow_id IN (SELECT workflow_id FROM workflow_ids)
    RETURNING 1
),
delete_message_send_events AS (
    DELETE FROM db_workflow.workflow_message_send_events
    WHERE workflow_id IN (SELECT workflow_id FROM workflow_ids)
    RETURNING 1
),
delete_loop_events AS (
    DELETE FROM db_workflow.workflow_loop_events
    WHERE workflow_id IN (SELECT workflow_id FROM workflow_ids)
    RETURNING 1
),
delete_activity_errors AS (
    DELETE FROM db_workflow.workflow_activity_errors
    WHERE workflow_id IN (SELECT workflow_id FROM workflow_ids)
    RETURNING 1
)
DELETE FROM db_workflow.workflows
WHERE workflow_id IN (SELECT workflow_id FROM workflow_ids)
RETURNING 1;
```

## Misc

```sql
select *
from db_workflow.workflows
where workflow_name = 'backend'
order by create_ts desc;
```

```sql
select workflow_activity_errors.*
from db_workflow.workflows
inner join db_workflow.workflow_activity_errors using (workflow_id)
where workflows.workflow_name = 'backend';
```
