# Redis

## What is Redis?

Redis is an in-memory, scriptable database used for operating on multiple types of data structures in
realtime. Redis can be used as a cache or for managing high-frequency realtime operations with guaranteed
consistency.

## What do we use Redis for?

We use Redis Streams for our durable consumers (similar to Kafka), for caching content, rate limiting API
requests, and in-memory databases like the matchmaker/parties/etc.

We run Redis in two main places: an ephemera cache used for data we don't care about persisting and a
persistent pool used for data we want to make sure never gets lost.
