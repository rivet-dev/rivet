# Chirp

## What is Chirp?

Chirp is the system used to communicate between Rivet services. It's built on top of NATS and Redis Streams. All communication over Chirp is encoded with Protobuf.

## Communication Types

### Remote Procedure Call (RPC)

RPC is useful for requests that only read data and don't mutate any database or have any permanent changes. This is similar to cases when you would use a `GET` HTTP request in traditional REST services.

See the `interface.proto` for all RPC services for their `Request` and `Response` types.

An example of an RPC service is `user-get` since it only reads data from a database.

RPC services will not be tried again if they fail. The caller of the RPC service has to wait for the RPC call to finish.

RPC calls are sent over NATS.

Messages will automatically fail if recursion is detected.

### Message & Consumers

Messages are a type of request that will be retried until completed successfully. Messages are sent and do not have a response. Messages are used for anything that mutates a database. See the `interface.proto` for all message services for their `Message` type and the `message.subject.parameters` property in `Service.toml` for the parameters the message gets dispatched to.

An example of a message is `msg-user-create` with the corresponding consumer `user-worker/src/workers/create.rs` which writes the user to the database.

To get a response from a message is processed, there needs to be another type of message. These are often named `msg-my-message-complete`. If a service has a valid failure state, there may also be a message type named `msg-my-message-fail`.

In order to listen for the response, the clients must publish a message then listen for the `*-complete` message to be published.

There can be multiple types of consumers for a specific type of message. For example, there might be a second consumer called `user-count` that also consumes `msg-user-create` and increments a counter for all users that are created.

Messages can also be chained together. For example, there might be a third consumer called `user-welcome-send` that consumes `msg-user-create-complete` and sends the user a chat message welcoming them to Rivet.

Messages are automatically cancelled if recursion is detected.
