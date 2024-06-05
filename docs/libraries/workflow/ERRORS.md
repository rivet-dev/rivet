# Errors

## Retries

Only errors from inside of activities will be retried. Errors thrown in the workflow body will not be retried
because they will never succeed (the state is consistent up the point of error).

## Workflow errors

Sub workflow errors cannot be caught because it's up to the workflow to handle its own errors gracefully.

We return OK responses from workflows for failure cases we explicitly handle (e.g. linode server provision
cleaning itself up)
