# Services

Bolt provides a concept of _services_ as a declarative way of defining what resources are required. Services
aren't always a unit of code; they can also be databases that need to provisioned or libraries used by other
services.

See `lib/bolt/config/src/service.rs` to jump straight to the source.

## Service types

### Operation

Simple request/response functions. If an operation fails, it will return the error. (See `GlobalError`.)

These should be used for anything that reads from a database. Anything that's purely logic should be under the
`util` library.

Operations are exposed as Rust libraries. They are called using the `op!` macro. Under the hood, these are
simple function calls, but are abstracted so they can be separate processes communicated using RPC if needed.

### Consumer

Similar to operations, but reads from a durable stream and has no response. If a consumer fails, it will be
retried indefinitely.

These should be used for anything that writes to a database.

Consumers are exposed as Rust libraries and are ran in the `monolith-worker` service. Messages can be
published using the `msg!` macro.

### API

Hosts API endpoints for communication with the outside world.

These should only call operations & publish messages. These should not contain any communication with the
database.

APIs are Rust binaries and ran as Kubernetes deployments with an exposed network interface. APIs can be
configured with routes to automatically be exposed publicly using Traefik.

### API routes

API routes are exposed as a library. These are imported by API services to expose publicly.

### Headless

Headless services are Rust binaries and ran as Kubernetes deployments. They're called "headless" because they
have no exposed network ports.

Useful for supporting infrastructure (such as log shipping) or adapting incoming data for Chirp services (such
as consuming Nomad events).

### Oneshot

Services that run once and only once. Useful for complex migrations.

Oneshot services are Rust binaries and ran as Kubernetes jobs.

### Periodic

Rust binaries deployed as a Kubernetes CRON job.

### Database

Creates a CockroachDB database. Contains migrations for this database that are automatically ran on
`bolt db migrate up`. See `bolt db --help` for more information.

### Cache

A Redis cache.

Redis databases are split between two clusters: a persistent database (with AOF persistence, no eviction) and
an ephemeral database (with LRU eviction).

There is no isolation between Redis services at the moment. This will change in the future with Redis ACL
support.
