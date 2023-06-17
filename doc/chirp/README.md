# Chirp

## What is Chirp?

Chirp is the system used to communicate between Rivet services. It's built on top of NATS and Redis Streams. All communication over Chirp is encoded with Protobuf.

## Service types

**TLDR**

- Operations don't mutate databases and can fail. Think of them similar HTTP `GET` requests.
- Workers make changes to databases and cannot fail. Think of them similar to HTTP `POST`/`PUT`/`DELETE` requests.

### Operations

_Often referred to as `ops`._

Operations are the most common type of service in Rivet.

Operations are used for requests that don't have permanent side effects (e.g. write to a database, making destructive API calls). They're commonly used for "getters" that execute database queries.

**Writing operations**

1. Create the Protobuf interface under `svc/pkg/*/types/my_operation.proto`
2. Create a library under `svc/pkg/*/ops/my-operation`
3. Write the operation body under `src/main.rs`
4. Write tests for the operation under `tests/`

**Calling operations**

1. Add a dependency to the operation in the service's `Cargo.toml`
2. Call the operation using `op!([ctx] my_operation {}).await?`

**Operations as libraries**

Rivet is designed around the philosophy of "build libraries, not microservices."

Each operation is an independent Rust micro-library that depends on other operations as libraries. When you see `op!` used in the code, it's calling a plan old function under the hood.

This provides the benefits of explicit isolation & testability of each operation without creating complicated & wasteful systems for microservices.

**Error handling**

Operations can return errors which will be propagated up the call stack. These get converted in to API errors if originating from an API request.

See the [error handling guide](/docs/chirp/ERROR_HANDLING.md) for more details.

### Workers

Workers are used for two main use cases:

- Performing operations that have permanent side effects (e.g. writing to a database, making destructive API calls)
- Consuming & responding to events (e.g. executing code when a user follows another user)

**Writing workers**

You'll usually need to create a new message for this worker. Do this first.

1. Create a worker under `svc/pkg/*/worker/src/workers/my_worker.rs`
2. Register the worker under `svc/pkg/*/worker/src/main.rs`

**Messages**

Workers are triggered by (i.e. "consume") messages through an event-based architecture.

Most workflows inside of Rivet are performed using a [choreography](https://solace.com/blog/microservices-choreography-vs-orchestration/).

This has many benefits, among which are:

- **Interoperability & extensibility** Workers can hook in to events from other parts of the code to add additional functionality, without modifying other services. For example:
    - The Rivet matchmaker is built on top of the abstract job event system without the job package knowing anything about the matchmaker.
    - The Rivet party system hooks in to the matchmaker event lifecycle to provide extra functionality without modifying the matchmaker at all.
- **Resilience** A lot of things can cause services to fail, like database failures, buggy deploys, and unexpected panics. Choreographed systems can recover from failures because they are stateless. As opposed to orchestration with a master server, which can crash and cause systems to fail.
- **Real-time by default** Since every step of a process is triggered by an event, systems are able to display real time results easily by hooking in to events from API services.
- **Simplicity** Event-based architectures has purely functional consumers with a clear input, output, and explicit list of messages it can publish. This makes it easy to determine what a service can do and how it can fail.

**Queuing**

Workers are processed in a queue. This makes them suitable for expensive and long-lasting operations.

**Error handling**

Workers cannot return errors. If a worker throws an error, then the worker will be retried with exponential back off until it succeeds. 

If you want to return an error from a worker, you need to create an error message type for the worker (e.g. `svc/pkg/team/types/msg/create-fail.proto`) and explicitly handle errors. Internal errors like database errors should not be caught, since workers should retry these types of requests.

See the [error handling guide](/docs/chirp/ERROR_HANDLING.md) for more details.

**Completion messages**

It's a common pattern to publish a separate completion message when a worker finishes.

For example, the `user-create` worker publishes the `msg-user-create-complete` message once complete. API servers consume this message to know when to return a `200 OK` from the request.

### Messages

Messages are a used to represent events or to trigger workers.

**Publishing messages**

Messages can be published using the `msg!` macro.

Messages are encode to Protobuf blobs that get written to both Redis Streams and NATS.

**Subscribing to messages**

Services can subscribe to messages by using the `subscribe!` macro.

This subscribes to the NATS topic to receive the message in realtime.

**Workers for consuming messages**

Workers can be created to consume messages. For example, a `user-create` worker can be created to consume the `msg-user-create` and the publish the `msg-user-create-complete` message.

## Service sizes

Services are designed to be as small as possible.

Refrain from creating monolithic services that do everything with a complicated request.

This helps encourage thorough unit tests, isolation & reproducibility of errors, and makes services easier to comprehend.
