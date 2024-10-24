# Why Redis?

## Motivation

Rivet has a lot of realtime workloads that require complex operations over small amounts of data.

For example:

- Matchmaking
- Parties
- User presence
- Caching

## Requirements

- In memory
- Support UDF
- Failover support
- Sharding support
- Strongly consistent
- High performance

## Alternatives

### Redis Stack

Redis Stack is licensed under the SSPL license, so we must stick with the vanilla Redis distribution.

Companies like Amazon & Upstash manage forks of RedisJSON before it was relicensed to SSPL. We've considered
doing this for Rivet too, but we don't have the manpower to maintain it and diagnose issues.

### Redis Raft

> TODO

### Cassandra & ScyllaDB & DataStax & Aerospike

> TODO

### KeyDB

KeyDB looks like it solves a lot of problems that comes with native Redis. It also provides a V8-based
scripting engine that would be much more enjoyable to use than Redis's built-in Lua scripting.

However, vanilla Redis is a battle tested option. It's unsure how KeyDB will continue under Snapchat's
ownership.

The option to migrate to KeyDB as a drop-in replacement is always available, but migrating back is likely more
difficult.

### DragonflyDB

Everything about DragonflyDB looks like the dream in-memory database on paper, but the company is too young to
consider at a startup. The risk of diagnosing nasty database bugs is much higher than the benefit of the
features it provides.

The option to
[migrate to DragonflyDB](https://www.dragonflydb.io/blog/migrating-from-a-redis-cluster-to-a-dragonfly-on-a-single-node)
as a drop-in replacement is always available, but migrating back is likely more difficult.

I'm excited to see what Dragonfly has in store.

### OLTP database

OLTP workloads have too much overhead for our use cases.

### OLAP database

OLAP databases don't provide the consistency needed for our use cases. Some OLAP databases offer CAS or some
form of transactions (e.g. Cassandra's LWT), but it's often limiting & often does not handle high amounts of
contention well.

## Shortcomings

> TODO
