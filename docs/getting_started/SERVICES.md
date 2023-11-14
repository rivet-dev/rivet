# Services

Bolt provides a concept of _services_ to split up components of Rivet's architecture. See `lib/bolt/config/src/service.rs` to jump straight to the source.

## Service types

### Operation

Simple request/response functions. If an operation fails, it will return the error. (See `GlobalError`.)

These should be used for anything that reads from a database. Anything that's purely logic should be under the `util` library.

Called using the `op!` macro. Under the hood, these are simple function calls, but are abstracted so they can be separate processes communicated using RPC if needed.

### Consumer

Similar to operations, but reads from a durable stream and has no response. If a consumer fails, it will be retried indefinitely.

These should be used for anything that writes to a database.

Messages can be published using the `msg!` macro.

### API

Hosts API endpoints for communication with the outside world.

These should only call operations & publish messages. These should not contain any communication with the database.

### API routes

Routes exposed as a library. These are imported by _API_ services and hosted on the main router.

### Headless

Services that don't allow any incoming network connections. Useful for supporting infrastructure (such as log shipping) or adapting incoming data for Chirp services (such as consuming Nomad events).

### Oneshot

Services that run once and only once. Useful for complex migrations.

### Periodic

CRON jobs.

### Database

Creates a CockroachDB database. Contains migrations for this database.

### Cache

A Redis cache. Currently a noop. Redis databases are split between two clusters: a persistent database (with AOF persistence, no eviction) and an ephemeral database (with LRU eviction).

