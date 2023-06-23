# Why Nebula?

## Motivation

Rivet is built to run across multiple clouds and on self-hosted instances with completely different networking configurations. Managing security of routing traffic between them became a mess of firewall rules without any isolation between them.

This was fine to start, since everything runs in a VLAN and we could just cross our fingers that there was no malicious code running inside our network. However, once we moved our job nodes outside of our VLAN for security reasons and tried to expose the minimal internal servers publicly (i.e. Nomad RPC for connecting nodes), we started exposing surface area for attacks and DoS publicly to the internet for some of our most essential services. We could _try_ to manage a massive list of IP whitelists to these ports, but this won't scale and is very error prone.

This made it clear that we need a self-hosted solution for dealing with security groups and firewalls and make it so we don't have to expose anything to the WAN and ideally not even to the LAN.

## Requirements

-   Completely self hosted, since each of our game servers would be counted as a node in any pricing model
-   Easy to run
-   Built as a mesh network for P2P communication
-   Resilient to failure
-   Doesn't require a public port to be open
-   Allows for running services that require clients to connect to arbitrary IPs (usually because of gossip protocol)
    -   Nomad
    -   Consul
    -   NATS
-   Allows for configuring access rules between nodes

## Alternatives

### Cloudflare Access

**Initial approach**

A lot of our initial approach was built on Cloudflare Access.

We hit a roadblock when trying to expose Nomad RPC over WAN. Because Nomad uses a gossip protocol, couldn't run the RPC connections through a tunnel since it tried to use the advertized address to make clients connect to other nodes.

i.e. we'd open and connect to a Nomad RPC tunnel running in 127.0.0.1:1234, but then Nomad would make the client connect to the advertised address at 1.2.3.4:4647 which was behind a firewall

**Closed source**

It was fine that this can't be self-hosted since we already pay for Cloudflare and rely on it heavily for our ingress traffic.

### Consul Connect/other service mesh

Service meshes suffer from the same issue as Cloudflare Access. We can't use services that rely on having clients connect to arbitrary public IPs. See the list of services that can't be ran using service meshes under _Requirements_.

### WireGuard

WireGuard likely would've worked.

**Not designed for mesh networking**

However, Nebula was built from the ground up to work like a mesh network, while WireGuard was built like a VPN. With WireGuard, you have to tell each peer about other peers.

To run WireGuard as a mesh, you can to use [wg-meshconf](https://github.com/k4yt3x/wg-meshconf) to automate the configurations, but this is significantly more complicated than Nebula.

**Fan out**

We also may have a very large "fan out" because we use NATS and often other databases which relies on sharding, so not using a proxy makes more sense for us.

Licensed under GPL, which is not inherently a problem, but it's preferred to use permissive OSS software.

The main advantage for WireGuard is that it has a large community and a lot of tooling around it.

Resources:

-   [Creating a Fast, Secure, Location Agnostic Mesh Network with Nebula - Ryan Huber](https://youtu.be/qy2cgqglt3o?t=1305)

### VPC/VLAN

We incorporated Nebula to enable secure traffic over WAN outside of the network's VPC/VLAN.

We still instruct Nebula to prefer traffic over LAN whenever possible. See [`preferred_ranges`](https://nebula.defined.net/docs/config/preferred-ranges/).

### ZeroTier

Achieves a similar goal to Nebula.

Likely would have worked fine for us, but Nebula was easy to get up and running, has worked flawlessly, and has already been proven at large scales.

It's worth noting that Nebula was spun out of Slack and is [now managed](https://www.defined.net/blog/open-for-business/) by [Defined](https://www.defined.net/) which aims to have a similar SaaS model to ZeroTier.

### Tailscale

[Not self-hostable.](https://tailscale.com/pricing/) For 1.5k nodes, it'd be at least $630/mo, i.e. an overhead of an extra $0.42/node which cuts in to our margins. That's without counting users and routers.

### NetMaker

It's licensed under SSPL, which is a non-starter since we need to self-host it.

Company is still too young to adopt for fundamental infrastructure.

Runs on top of WireGuard, so inherits the same pain points.

## Shortcomings

### No support for SSO

Other services like Cloudflare Access and ZeroTier allow for SSO which makes it easy for us to expose our internal services.

We can still use Cloudflare Access to provide SSO alongside Nebula.

### Small community

Compared to most of the other tools mentioned here, Nebula as the smallest and youngest community. However, it's used at all of Slack and likely other companies, so this is not a big concern.

> The reception has been astoundingly positive. As of this writing, the project has nearly 11,000 stars on GitHub and is trusted by a large base of enthusiastic users, along with multiple Fortune 50 companies.
> [Source](https://www.defined.net/blog/open-for-business/#user-content-fn-0)
