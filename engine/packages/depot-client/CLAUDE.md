# Depot Client

- Communicate between async runtime tasks and the SQLite worker thread through channels. Do not call back into the Tokio runtime from the SQLite thread.
