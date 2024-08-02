# Signals

## Tagged signals

Tagged signals are consumed on a first-come-first-serve basis because a single signal being consumed by more
than one workflow is not a supported design pattern. To work around this, consume the signal by a workflow
then publish multiple signals from that workflow.
