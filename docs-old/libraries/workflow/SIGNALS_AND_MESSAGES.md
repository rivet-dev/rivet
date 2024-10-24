# Signals

## Tagged signals

Tagged signals are consumed on a first-come-first-serve basis because a single signal being consumed by more
than one workflow is not a supported design pattern. To work around this, consume the signal by a workflow
then publish multiple signals from that workflow.

# Choosing Between Signals and Messages

> **Note**: non-workflow ecosystem is API layer, standalone, operations, old workers

## Signal

- Sending data from the non-workflow ecosystem to the workflow ecosystem
- Sending data from the workflow ecosystem to somewhere else in the workflow ecosystem

## Message

- Sending data from the workflow ecosystem to the non-workflow ecosystem

## Both Signals and Messages

Sometimes you may need to listen for a particular event in the workflow system and the non-workflow ecosystem.
In this case you can publish both a signal and a message (you can derive `signal` and `message` on the same
struct to make this easier). Just remember: signals can only be consumed once.

Both messages and signals are meant to be payloads with a specific recipient. They are not meant to be
published without an intended target (i.e. any listener can consume).
