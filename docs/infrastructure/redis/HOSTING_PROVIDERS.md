# Hosting Providers

## Requirements

We want the impossible. We want something that scales on pricing but doesn't require us to pay up front.

- High QPS (implied by Redis Cluster support)
- Compatible with vanilla Redis (including EVAL)
- AOF persistence
- Reasonably affordable

### Nice to haves

- Redis Cluster
- Autoscaling

## Options

### Redis Cloud

**Pros**

Managed by the same company that develops Redis OSS, so it should be the most reliable.

Lets us choose our cloud provider.

**Cons**

Redis Cloud is absurdly expensive.

Redis offering is still on 6.2, even though 7 has been out for a long time.

They abruptly shut down a Rivet account that caused a complete outage:

> We have noticed that your current usage of Redis Enterprise Cloud - Fixed Plan is causing network congestion and high network costs for Redis. As a result, in accordance with the terms of your subscription, we sent you an email and blocked your account.

Will not be returning as a customer after this incident.

### Aiven

**Pros**

Supports multiple clouds. Priced fairly.

**Cons**

Does not support Redis Cluster.

### DragonflyDB

**Pros**

Fast single-node speeds.

Supports clustered mode.

Their benchmarks promise significantly higher QPS than we already require. While we rely on Redis Cluster's linear scalability, it would likley be possible ot run DragonflyDB of a single machine.

Prometheus is built in.

**Cons**

Uneasy with using a re-implementaiton complicated mechanisms. Very likely we'll run in to implementation differences with with either streams or clustering.

BSL license _should_ be compatible with Apache 2.0, but not reassuring.

Only supports snapshotting, not AOF persistence.

[Redis contests these benchmarks.](https://redis.com/blog/redis-architecture-13-years-later/)

### AWS ElastiCache & MemoryDB

**Pros**

Comes with AWS's reliability guarantees.

**Cons**

Insanely expensive. Quoted ~$1.5k for a 6 node (3 nodes + 3 replicas) cluster.

### Upstash

**Pros**

Serverless offering is cheaper for low-volume setups and automatically scales.

Fly.io & Vercel have bet their companies on Upstash.

**Cons**

Seems like a bleeding edge architecture, would rather stick with a boring vanilla Redis setup. Likely to have weird sharp edges that we'll run in to considering how heavily we rely on Redis.

Upstash's QPS are weirdly low. They force you to ugprade to enterprise at 10k QPS. Paying extra for Upstash is not a problem, but it's not reassuring how small workload they consider to be "enterprise."

Does not let us choose our own cloud provider, so there's an extra 2 ms latency between the client & server. Caching services services need to be in the same datacenter.

Incredibly expensive. For a 1k QPS load, we would either be paying (a) $43.2k/mo on "Pay as you go" or (b) $280/mo on "Pro 2K." Aiven can easily match this at < $100/mo.

### Self-hosted K8S

**Pros**

Supports clustering. Cheap. Flexible. Scalable. Easy licensing.

**Cons**

Managed solutions can probably provide better uptime than we can internally.

Does not provide automatic backups.

### Self-hosted + VPA

**Pros**

See all above.

We already pay the TCO for Karpenter, which is really good at dealing with situations like this.

Could help scale to meet spiky traffic.

Saves cost on smaller clusters.

Allows us to absorb load and manualy adjust shard count as needed.

**Cons**

Self-hosting is already prone to go wrong. 

There is no prior art for a VPA on Redis cluster.

There is a HPA available for [vanilla Redis](https://artifacthub.io/packages/helm/bitnami/redis) for scaling replicas. This doesn't help with Redis Cluster, though.

Would be interesting to [experiement with HPA for Redis Cluster](https://medium.com/swlh/scaling-redis-cluster-via-kubernetes-horizontal-pod-autoscaler-852541c01b29), but that is more prone to failure than a VPA.

### Hybrid: Self-hosted Ephemeral + Upstash Persistence

**Pros**

Our high QPS operations are using ephemeral storage, so we can self-host this safely.

Our low-latency operations are using ephemeral storage which will be self-hosted, so our latency for Upstash workfloads will be lower.

We don't need replicas for caches, so we can save money here.

Upstash lets us autoscale according to our usage, which leads to being cheaper overall.

**Cons**

See preivous cons related to Upstash.

Upstash is still extreamly expensive.

## Solution

Self-hosted K8S, potentially with VPA in the future.

This is the most cost-effective and scalable solution. It comes with more difficulty in managing & monitoring Redis.

