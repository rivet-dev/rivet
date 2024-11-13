# Glossary

## Worker

A process that queries for pending workflows with a specific filter. Filter is based on which workflows are
registered in the given worker's registry. The queried workflows are run on the same machine as the worker but
given their own thread.

## Registry

A collection of registered workflows. This is solely used for the worker to fetch workflows from the database.

## Workflow

A series of fallible executions of code (also known as activities), signal listeners, signal transmitters, or
sub workflow triggers.

Workflows can be though of as an outline or a list of tasks. The code defining a workflow only specifies what
items should be ran; There is no complex logic (e.g. database queries) running within the top level of the
workflow.

Upon an activity failure, workflow code can be reran without duplicate side effects because activities are
cached and re-read after they succeed.

## Activity

A block of code that can fail. This cannot trigger other workflows or activities, but it can call operations.
Activities are retried by workflows when they fail or replayed when they succeed but a later part of the
workflow fails.

When choosing between a workflow and an activity:

- Choose a workflow when there are multiple steps that need to be individually retried upon failure.
- Choose an activity when there is only one chunk of retryable code that needs to be executed.

## Operation

Effectively a native rust function. Can fail or not fail. Used for widely used operations like fetching a
user. Operations cannot be called from workflows.

Examples include:

- most `get` operations (`user-get`)
- any complex logic you'd want in it's own function (fetching some http data and parsing it)

Operations are not required; all of their functionality can be put into an activity instead.

## Tags

Tags are JSON blobs associated with either workflows or signals. Tags are not meant to be very abstract; i.e.
they should be unique.

## Signal

A payload sent to a specific workflow from anywhere else in the codebase. The workflow must be listening for
this signal for it to be picked up, otherwise it will stay in the database indefinitely until consumed by a
workflow. Signals do not have a response; another signal must be sent back from the workflow and listened to
by the sender.

### Differences between message

Signals are like messages that can only be consumed by workflows and can only be consumed once.

## Tagged Signal

Same as a signal except it is sent with a JSON blob as its "tags" instead of to a specific workflow. Any
workflow with tags that are a superset of the signals tags will consume the signal. Note that tagged signals
are consumed on a first-come-first-serve basis, meaning if there are two workflows that both have a superset
of the signal's tags, only one will receive the signal.

See [the signals document](./SIGNALS.md).

## Join Signal

A "one of" for signal listening. Allows for listening to multiple signals at once and receiving the first one
that gets sent.

## Message

A payload that can be sent out of a workflow. Includes a JSON blob for tags which can be subscribed to with a
subscription.

### Differences between signal

Messages are like signals that can be only consumed by non workflows and can be consumed by multiple
listeners.

## Subscription

An entity that waits for messages with the same (not a superset/subset) tags as itself. Upon receiving a
message, the message will be returned and the developer can choose to continue to listen for more messages.

## Tail

Reads the last message without waiting. If none exists (all previous messages expired), `None` is returned.

## Tail w/ Anchor

Reads the earliest message after the given anchor timestamp or waits for one to be published if none exist.

## Workflow Event

An action that gets executed in a workflow. An event can be a:

- Activity
- Received signal
- Dispatched sub-workflow

Events store the output from activities and are used to ensure activities are ran only once.

## Workflow Event History

List of events that have executed in this workflow. These are used in replays to verify that the workflow has
not changed to an invalid state.

## Workflow Replay

After the first run of a workflow, subsequent runs will replay the activities and compare against the event
history. If an activity has already been ran successfully, the activity will not actually run any code and
instead use the output from the previous run.

## Workflow Wake Condition

If a workflow is not currently running an activity, wake conditions define when the workflow should be ran
again.

The available conditions are:

- **Immediately** Run immediately by the first available node
- **Deadline** Run at a given timestamp.
- **Signal** Run once any one of the listed signals is received.
- **Sub workflow** Run once the given sub workflow is completed.
