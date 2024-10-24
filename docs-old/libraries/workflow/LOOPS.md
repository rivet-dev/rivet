# Loops

TODO

## Differences between "Continue As New"

https://docs.temporal.io/develop/go/continue-as-new

Continue As New effectively wipes the entire history of the workflow, allowing you to start from scratch when
needed.

With loops, only the history of previous completed iterations of the loop is forgotten. This is because it is
assumed there are no side effects from the loop, meaning previous iterations have no effect on the workflow.
