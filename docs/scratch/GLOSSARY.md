# Glossary

## Chirp

-   service: a small unit of code that provides a small function
-   worker: a type of service that's tightly integrated with chirp; can either be
    an rpc endpoint or consumer
-   rpc: ephemeral requests that; these are intended to be used for non-mutating
    requests

### Consumers

-   consumer: a worker that reads messages from a durable stream; these are
    intended for requests that mutate state
-   message: a event that gets published to a stream and to the pubsub broker
-   topic: synonamous with the name of the message; used to define the keys for
    publishing message events and storage keys
-   parameters: variable data associated with a topic for a given message
-   subject: a specific topic & parameter combination (including wildcards), these are used for listening for events over the pubsub server
-   (durable) stream: the data structure (stored in Redis at the moment) used by consumer workers to read and ack messages reliably
