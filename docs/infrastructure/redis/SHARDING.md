# Redis Sharding

When running Redis in [cluster mode](https://redis.io/docs/management/scaling/), managing which key ends up on which shard becomes important for piped commands<sup>†</sup> and Redis scripts.

Redis Cluster comes with a feature known as [hash tags](https://redis.io/docs/reference/cluster-spec/#hash-tags) which allow the user to dictate where keys end up in relation to each other. Keys with the same hash tag (for example, `{foo}.bar` and `{foo}.baz`) will end up on the same shard.

The reason this is required for piped commands is because of how piped commands handle the [`MOVED`](https://redis.io/docs/reference/cluster-spec/#moved-redirection) error response from Redis. If the Redis connection is currently connected to a shard that does not have the key that the give command needs to act on, Redis Cluster will respond with a `MOVED` error telling it where to go. This works well when executing single commands at a time, but in piped commands it is impossible to execute multiple commands in a pipe if all of the keys in said commands are not on the same shard.

> † Commands using https://docs.rs/redis/latest/redis/struct.Pipeline.html

## Choosing the right hash tag

The two criteria for picking a hash tag are:

-   What makes sense to run on the same machine (i.e. same pipe)
-   What helps distribute load well

Simple example:

```
user_presence:key:user_presence
```

would turn in to

```
{{user:{user_id}}}:presence
```

_(Using rust formatting string syntax)_
