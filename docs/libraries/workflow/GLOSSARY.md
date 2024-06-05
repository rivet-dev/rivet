TODO

# Glossary

## Worker

A process that's running workflows.

There are usually multiple workers running at the same time.

## Workflow

A series of activies to be ran together.

The code defining a workflow only specifies what activites to be ran. There is no complex logic (e.g. database queries) running within workflows.

Workflow code can be reran multiple times to replay a workflow.

## Workflow State

Persistated data about a workflow.

## Workflow Run

An instance of a node running a workflow. If re-running a workflow, it will be replaying events.

## Workflow Event

An action that gets executed in a workflow. An event can be a:

-   Activity

Events store the output from activities and are used to ensure activites are ran only once.

## Workflow Event History

List of events that have executed in this workflow. These are used in replays to verify that the workflow has not changed to an invalid state.

## Workflow Replay

After the first run of a workflow, all runs will replay the activities and compare against the event history. If an activity has already been ran successfully, the activity will be skipped in the replay and use the output from the previous run.

## Workflow Wake Condition

If a workflow is not currently running an activity, wake conditions define when the workflow should be ran again.

The available conditions are:

-   **Immediately** Run immediately by the first available node
-   **Deadline** Run at a given timesetamp.

## Activity

A unit of code to run within a workflow.

Activities can fail and will be retried accoriding to the retry policy of the workflow.
