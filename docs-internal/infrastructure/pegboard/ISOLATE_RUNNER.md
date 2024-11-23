# Error handling

There are several layers of error handling in the isolate runner. Going from the isolate runtime to the main
thread:

1. Js runtime errors are caught in various places in `run_inner` in `src/isolate.rs`. These are written to the
   stderr log stream and visible to users on the dashboard. No error is thrown from the function.
2. Errors besides js runtime errors are thrown by `run_inner` and caught by `run`. An error line is written to
   the stderr log stream stating that a fatal error has occurred. An error code of 1 is returned by the `run`
   function and no error is thrown by `run`.
3. If any error is thrown by the `run` function (likely during setup or cleanup), it is caught by the tokio
   task (in `src/main.rs`) which is watching the thread where the `run` function is running from. Upon any
   error, it logs the error and sends a message to the `fatal_tx` channel.
4. The `main` function in `src/main.rs` is a `tokio::select!` on the WS retry loop and `fatal_tx`. If any
   message is received on `fatal_tx`, an error code of 1 is written and the program throws "Fatal error".
5. Besides messages from `fatal_tx`, the main function can fail during setup (redirecting logs, reading
   config, writing pid, etc) or from fatal errors from the WS connection. This includes bad packets or failed
   socket sends. It is intended to fail for these cases, but should automatically handle retryable errors like
   the socket closing.
