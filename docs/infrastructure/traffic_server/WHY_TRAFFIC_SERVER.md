# Why Traffic Server?

## Motivation

Rivet needs a caching forward proxy for many internal components.

## Requirements

- Self-hostable
- High performance
- Pull-through cache
- Serve cold requests while caching at the same time
    - Instead of downloading the file to the cache then serving, effectively doubling the download time

### Nice to haves

- Cache hierarchy for sharded hashes
    - Consul's [hash based routing](https://developer.hashicorp.com/consul/docs/connect/config-entries/service-resolver#hashpolicies) can be used if not supported natively

## Alternatives

### Varnish

Varnish

> TODO

### Squid

Squid does not support serving files at the same time as caching them.

Squid is also notoriously slow. Its architecture [directly led](https://info.varnish-software.com/blog/varnish-or-squid) to the development of Squid.

### NGINX Plus caching

NGINX has a caching module, but you must pay for NGINX Plus to use it. NGINX Plus [starts at $3,675 per module](http://web.archive.org/web/20230601061826/https://www.nginx.com/pricing/). This is a non-starter for an open source project and is _far_ too expensive for an early stage startup.

> TODO

### S3 global replication

S3 global replication provides many of the same performance benefits that we get out of Traffic Server.

However, it requires us to replicate our _entire_ S3 buckets which is incredibly expensive, considering the amount of Docker images & CDN assets we store.

The requests going straight to S3 will be very expensive, we'll be billed per request. Traffic Server is within the same datacenter and costs nothing to request a prewarmed file.

## Shortcomings

- Traffic Server often fails in non-obvious ways without logging, you need to know what you're doing before using it
- While we don't need to Dockerize it at the moment, Traffic Server has a slew of bugs inside of Docker
- Development is sluggish

## Related links

- [Hacker News](https://news.ycombinator.com/item?id=10983331)
