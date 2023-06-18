# Why we don't support IPv6

**TLDR: We haven't had the time to deal with it yet**

## Ingress

Ingress IPv6 should be fairly straightforward to implement for Rivet.

As long as we have an IPv4 address available for our load balancers, we can use IPv6 for all other servers without issue.

### Counting unique IPs

At the moment, we use a simple IP counting system in the matchmaker that works well enough for IPv4. However, we need to support counting IPv6 by larger IP blocks, since each residential address may have [18 quintillion IP unique IP addresses](https://www.computerworld.com/article/2729027/comcast-is-first-u-s--isp-to-offer-ipv6-to-home-gateway-users.html#:~:text=In%20a%20somewhat%20controversial%20move,or%2018%2C446%2C744%2C073%2C709%2C551%2C616%20to%20be%20exact.).

### Traefik rate limiting

Traefik does not [natively support IP blocks](https://doc.traefik.io/traefik/middlewares/http/ratelimit/#sourcecriterionipstrategy) for rate limiting, which we utilize.

## Internal

We should be able to use IPv6 between nodes at the moment with Consul, Nomad, and Nebula without issue.

### Nebula

Nebula does not support IPv6 for communicating over the VPN, so we must stick with IPv4 for now. This is OK.

It has no problem communicating between nodes using IPv6.
