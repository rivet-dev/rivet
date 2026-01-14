# Gasoline

Gasoline (at engine/packages/gasoline) is the durable execution engine running most persistent things on Rivet Engine.

Gasoline consists of:
- Workflows - Similar to the concept of actors. Can sleep (be removed from memory) when not in use
- Signals - Facilitates intercommunication between workflow <-> workflow and other services (such as api) -> workflow
- Messages - Ephemeral "fire-and-forget" communication between workflows -> other services
- Activities - Thin wrappers around native rust functions, each can be individually retried upon failure
- Operations - Thin wrappers around native rust functions. Provided for clean interop with the Gasoline ecosystem

## Workflows

Workflows consist of a series of durable steps. When a step is complete, its result is saved to database as workflow history. If a workflow encounters a step which requires waiting (i.e. a signal, or just sleeping) it will be removed from memory until that event happens or the sleep times out.

When a workflow is rewoken from database back into memory, all previous steps that already completed will be "replayed". This means they will not actually execute code or write anything to database, but instead use the data already in the database to mimic an actual response.

## Workflow Steps

All workflow steps are durable, meaning they are either NO-OPs or return the previously calculated result when the workflow replays.

These include:

- Signal - a durable packet of data sent to a workflow. Can have a timeout and ready batch size
- Message - an ephemeral packet of data sent from a workflow to a subscriber
- Activity - a thin wrapper around a bare function with automatic backoff retries
- Sub workflow - publishing another workflow and/or waiting for another workflow to complete before continuing
- Loop - allows running the same closure repeatedly while intelligently handling workflow history
- Sleep
- Removed events, version checks - advanced workflow history steps (see WORKFLOW_HISTORY.md)

You cannot run operations directly in the workflow body; they must be put in an activity first.

Some composition methods:

- Join - run multiple activities or closures in parallel
- Closure - create a branch in workflow history

### Activities

Activities are used to run user code; they are the meat and potatoes of the workflow. They should be composed in a way such that their failure and subsequent retry does not cause any side effects. In other words, each activity should be limited to 1 "action" that, when retried, will not be clobbered by previous executions of the same activity.

Pseudocode (bad composition):

- Activity 1
	- Transaction 1: Insert user row into database
	- Transaction 2: Insert user id into group table

If this activity were to fail on transaction 2, it will be retried with backoff. However, because the user row already exists, retrying will result in a database error. This activity will never succeed upon retry.

To remedy this, either:
	- Combine the queries into one transaction OR
	- Separate the transactions into 2 activities, one for each transaction

### Signals

Signals are the only form of communication between services (anything outside of workflows) -> workflow, as well as workflow -> workflow.

To send a signal you need:

- A workflow ID OR
- Tags that match an existing incomplete workflow

If the workflow is found, the signal is added to the workflows queue. Workflows can consume signals by using a `listen` step.

If a workflow completes with pending signals still in its queue, the signals are marked as "acknowledged" and essentially forgotten.

### Messages

Messages are ephemeral packets of data sent from workflows. They are intended as status updates for real time communication and not durable communication. It is ok for messages to not be consumed by any receiver.

Receiving a message requires subscribing to its name and a subset of its tags.

## Tags

Workflows, signals, and messages all use tags for convenience when their unique IDs are not available.

Signals can be sent to a workflow if you know its name and a subset of its tags.

Internally, it is more efficient to order signal tags in a manner of most unique to least unique:

- Given a workflow with tags:
	- namespace = foo
	- type = normal

The signal should be published with `namespace = foo` first, then `type = normal`