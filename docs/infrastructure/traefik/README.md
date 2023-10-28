# Traefik

## What is Traefik?

Traefik is an easy to configure and flexible load balancer. Treafik provides many different ways of configuring dynamically and many types of middlewares that let us maniuplate incoming traffic.

## What do we use Traefik for?

We run three main Treafik pools:

-   "proxied" which we use take traffic coming in from Cloudflare and route them to our API services
-   "unproxied" which we use for traffic sent directly to our cluster and route it to our CDN based on the games' CDN configurations
-   "job" which proxies WebSocket traffic sent to our game nodes and provides basic DoS protection

## How is Traefik configured?

_TODO_
