# Glossary

## Chirp

- service: An individual process/container that does something. See `bolt_config::project::serviceKind`.
- chirp client: A lightweight client to interface with Chirp services. Usually used from a _context_. See
  `chirp_client::ChirpClient`.
- context: Contexts can represent workers (see `chirp_worker::Message`), operations (see
  `rivet_operation::Operation`), or tests (see `chirp_worker::TestCtx`). See `rivet_operation::OperationCtx`
  for the core functionality.
- worker: A chunk of code that can process inputs from a stream of messsages/requests. These can be consumers
  or rpc endpoints (not currently used). There are usually multiple workers in a single service using a
  _worker group_. See `chirp_worker::Manager`.
- worker group: Runs multiple _workers_ in parallel in one process.
- consumer: A type of worker that consumes a stream of messages and has no response. Used for mutating code
  that _needs_ to succeed. Requests will be retried until succeeds.
- operation: Ephemeral requests. Used for non-mutating, ephemeral code. Represented as individual Rust
  libraries. See `rivet_operation::Operation`.

### Consumers

- consumer: a worker that reads messages from a durable stream; these are intended for requests that mutate
  state
- message: a event that gets published to a stream and to the pubsub broker
- topic: synonymous with the name of the message; used to define the keys for publishing message events and
  storage keys
- parameters: variable data associated with a topic for a given message
- subject: a specific topic & parameter combination (including wildcards), these are used for listening for
  events over the pubsub server
- (durable) stream: the data structure (stored in Redis at the moment) used by consumer workers to read and
  ack messages reliably
