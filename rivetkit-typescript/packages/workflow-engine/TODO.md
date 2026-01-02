## loops as checkpoints

- todo

## test migrating workflows

- run workfklow code a
- run workflow code b

## remove the internal signal queue

## observe workflow status

- add support for monitoring workflow state by emitting events from the workflow
- subscribe to events
- update all tests to listen for specific events
	- update tests to use snapshot testing based on these events

## ephemeral-by-default steps

- make steps ephemeral by default
- add helper fns for things like fetch, clients, etc that auto-flag a step as required to be durable
- can also opt-in to flag a step as durable
- tests:
	- default steps are ephemeral without config
	- opt-in durable steps flush immediately
	- helper wrappers mark steps durable
	- mixed ephemeral/durable sequences flush as expected

## rollback

- support rollback steps
- tests:
	- rollback executes in reverse order
	- rollback persists across restart
	- rollback skips completed steps when resumed
	- rollback respects abort/eviction

## misc

- remove workflow state in favor of actor state
- tests:
	- workflow state mirrors actor state transitions
	- cancelled workflows report actor state
	- storage no longer writes workflow state key

## types

- generic messages
- tests:
	- typed messages enforce payload shape
	- message serialization round-trips typed payloads
	- listen helpers preserve generic type inference

## otel

- tbd on what the trace id represents
- tbd on actions

