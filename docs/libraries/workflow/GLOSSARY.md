# Glossary

## Worker

A process that queries for pending workflows with a specific filter. Filter is based on which workflows are registered in the given worker's registry.
The queried workflows are run on the same machine as the worker but given their own thread.

## Registry

A collection of registered workflows. This is solely used for the worker to fetch workflows from the database.

## Workflow

A series of fallible executions of code (also known as activities), signal listeners, signal transmitters, or sub workflow triggers.

Workflows can be though of as a list of tasks. The code defining a workflow only specifies what items should be ran; There is no complex logic (e.g. database queries) running within the top level of the workflow.

Upon an activity failure, workflow code can be reran without duplicate side effects because activities are cached and re-read after they succeed.

## Activity

A block of code that can fail. This cannot trigger other workflows or activities, but it can call operations.
Activities are retried by workflows when they fail or replayed when they succeed but a later part of the
workflow fails.

## Operation

Effectively a native rust function. Can fail or not fail, used simply for tidiness (as you would with any other function).
Operations can only be called from activities, not from workflows.

Examples include:

-   most `get` operations (`user-get`)
-   any complex logic you'd want in it's own function (fetching some http data and parsing it)

Operations are not required; all of their functionality can be put into an activity instead.

## Workflow Event

An action that gets executed in a workflow. An event can be a:

-   Activity
-   Received signal
-   Dispatched sub-workflow

Events store the output from activities and are used to ensure activities are ran only once.

## Workflow Event History

List of events that have executed in this workflow. These are used in replays to verify that the workflow has not changed to an invalid state.

## Workflow Replay

After the first run of a workflow, subsequent runs will replay the activities and compare against the event history. If an activity has already been ran successfully, the activity will not actually run any code and instead use the output from the previous run.

## Workflow Wake Condition

If a workflow is not currently running an activity, wake conditions define when the workflow should be ran again.

The available conditions are:

-   **Immediately** Run immediately by the first available node
-   **Deadline** Run at a given timestamp.
-   **Signal** Run once any one of the listed signals is received.
-   **Sub workflow** Run once the given sub workflow is completed.
