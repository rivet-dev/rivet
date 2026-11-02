# Local blocking-operation ownership

LocalFileSystem dispatches filesystem I/O to Tokio blocking jobs. Dropping an async caller does not cancel those jobs. The Engine local adapter needs its directory and per-object ownership to survive actual I/O completion.

The patch retains an optional opaque guard inside mutating blocking closures. Concrete multipart uploads accept per-operation guards and retain their directory guard during staging cleanup. No storage format or remote-store behavior changes. The adapter owns guard policy; the dependency only carries lifetime ownership.

Regression: engine-context local_object_store tests queue a write behind a bounded blocking pool, cancel its caller, and require ownership to remain exclusive until the queued write completes.
