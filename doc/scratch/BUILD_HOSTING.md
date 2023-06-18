# Job Build Hosting

## Implementation

Job run builds are hosted publicly to the internet behind HTTP authentication in
order to enable flexibility in where Nomad clients are running.

When requesting a Docker build, it goes through these steps:

1. mm-lobby-create specifies the Nomad artifact URL, which includes HTTP auth
1. Nomad makes HTTP request for the artifact
1. Unproxied Traefik instance with HTTP authentication

We use an unproxied Traefik proxy in each region with an ATS

## Future Plans

We can't dynamically update firewalls to include the autoscaling job servers on
all of our VPS providers, so this is the best option available to securely host
Docker images.

In the future, we'll have a custom plugin for whitelisting incoming IPs in
addition to HTTP authentication in order to validate job server.

Additionally, we'll eventually choose the optimal regions to run ATS in so we're
not over-paying for block storage for ATS caching.

## Why not Consul Connect

We can't use Consul Connect to authenticate this since game nodes have extreamly
limited resources and can't run a Consul Connect instance.
