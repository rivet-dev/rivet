# Timeouts

Many load balancers have 60s configured as default timeout. Our API timeouts are designed to work within these bounds.

## Preface: How network reaches Rivet

```
Client -> Cloudflare -> NLB -> Traefik -> api-monolith
```

## Infra timeouts

These are the timeouts that our API servers are restricted to:

-   Cloudflare: 100s ([source](https://developers.cloudflare.com/support/troubleshooting/cloudflare-errors/troubleshooting-cloudflare-5xx-errors/#error-524-a-timeout-occurred))
    -   **Behavior** Returns a 524
    -   Cannot be configured unless paying for Cloudflare Enterprise
-   AWS NAT Gateway: 350 seconds idle (without keepalive) ([source](https://docs.aws.amazon.com/vpc/latest/userguide/nat-gateway-troubleshooting.html#nat-gateway-troubleshooting-timeout))
    -   **Behavior** Connection drop
-   AWS NLB: 350 seconds ([source](https://docs.aws.amazon.com/elasticloadbalancing/latest/network/network-load-balancers.html#connection-idle-timeout))
    -   **Behavior** Connection drop
-   Traefik: 60s ([source](https://github.com/rivet-gg/rivet/blob/c63067ce6e81f97b435e424e576fbd922b14f748/infra/tf/k8s_infra/traefik.tf#L65))
    -   **Behavior** _Unknown_
    -   Unlike the other timeouts, this is configurable by us

## Rivet API Timeouts

We use long polling (i.e. `watch_index`) to implement real time functionality. This means we need to be cautious about existing timeouts.

Current timeouts:

-   `api-helper`: 50s ([source](https://github.com/rivet-gg/rivet/blob/9811ae11656d63e26b4814fe15f7f852f5479a48/lib/api-helper/macros/src/lib.rs#L975))
    -   **Behavior** Returns `API_REQUEST_TIMEOUT`
    -   **Motivation** This gives a 10s budget for any other 60s timeout
-   `select_with_timeout!`: 40s ([source](https://github.com/rivet-gg/rivet/blob/9811ae11656d63e26b4814fe15f7f852f5479a48/lib/util/macros/src/lib.rs#L12))
    -   **Behavior** Timeout handled by API endpoint, usually 200
    -   **Motivation** This gives a 10s budget for any requests before/after the select statement
-   `tail!` and `tail_all!`: 40s (depending on `TailAllConfig`) ([source](https://github.com/rivet-gg/rivet/blob/9811ae11656d63e26b4814fe15f7f852f5479a48/lib/util/macros/src/lib.rs#L12))
    -   **Behavior** Timeout handled by API endpoint, usually 200
    -   **Motivation** This gives a 10s budget for any requests before/after the select statement

## Database connections

### CockroachDB

-   `idle_timeout` is set to 3 minutes, which is less than the NAT Gateway timeout
-   `test_before_acquire` is left as true in order to ensure we don't run in to timeouts, even though this adds significant overhead

### Redis

-   We ping the database manually every 15 seconds
-   Back off retries is set to infinity in order to ensure that `ConnectionManager` always returns to a valid state no matter the connection issues
    -   The current internal logic will cause the Redis connection to fail after 6 automatic disconnects, which will cause the cluster to fail if idle for too long

### Misc Resources

-   [Implementing long-running TCP Connections within VPC networking](https://aws.amazon.com/blogs/networking-and-content-delivery/implementing-long-running-tcp-connections-within-vpc-networking/)
-   [Introducing configurable Idle timeout for Connection tracking](https://aws.amazon.com/blogs/networking-and-content-delivery/introducing-configurable-idle-timeout-for-connection-tracking/) (this is intentionally not configured)
