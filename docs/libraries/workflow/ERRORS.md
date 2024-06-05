# Errors

Only errors from inside of activities will be retried. Errors thrown in the workflow body will not be retried because they will never succeed (the state is consistent up the point of error).
