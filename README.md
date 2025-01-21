<p align="center">
  <a href="https://rivet.gg">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="./.github/media/icon-text-white.svg" alt="Rivet">
      <img src="./.github/media/icon-text-black.svg" alt="Rivet">
    </picture>
  </a>
</p>


<p align="center">
  <p align="center"><b>Scalable. Stateful. Serverless.</b><br/>Rivet is the platform to build realtime, edge, or agent applications.No limitations of Redis or timeouts of Lambda.</p></p>
<h4 align="center">
  <a href="https://hub.rivet.gg">Rivet Cloud</a> |
  <a href="https://rivet.gg/docs/self-hosting">Self-Hosting</a> |
  <a href="https://rivet.gg/docs">Docs</a> |
  <a href="https://www.rivet.gg">Website</a>
</h4>
<p align="center">
  <a href="/LICENSE"><img alt="License Apache-2.0" src="https://img.shields.io/github/license/rivet-gg/rivet?style=flat-square"></a>
  <a href="https://github.com/rivet-gg/rivet/graphs/commit-activity"><img alt="GitHub commit activity" src="https://img.shields.io/github/commit-activity/m/rivet-gg/rivet?style=flat-square"/></a>
  <img alt="GitHub Discussions" src="https://img.shields.io/github/discussions/rivet-gg/rivet?style=flat-square">
  <a href="https://rivet.gg/discord"><img alt="Discord" src="https://img.shields.io/discord/822914074136018994?style=flat-square&label=discord"/></a>
   <a href="https://twitter.com/rivet_gg">
    <img src="https://img.shields.io/twitter/follow/rivet_gg?label=Follow" alt="Rivet Twitter" />
  </a>
</p>

![Code snippets](./.github/media/code.png)

Rivet comes with simple primitives to build your backend. Leverage Rivet Actors to build complex functionality with ease.

## Features

- [**State & Persistence**](https://rivet.gg/docs/state): State that feels like memory but works like storage. Ideal for dynamic, fast-moving apps.
- [**Remote Procedure Calls**](https://rivet.gg/docs/rpc): Lightweight messaging built for speed. Complete client/server type safety included.
- [**Runs Forever, Sleeps When Idle**](https://rivet.gg/docs/lifecycle): Always available, sleeps on network inactivity or timeouts, and wakes instantly on demand.
- [**Edge Networking**](https://rivet.gg/docs/edge): Automatically distribute your applications near your users for ultra-low latency.
- [**Fault Tolerance**](https://rivet.gg/docs/fault-tolerance): Ensure application & state resilience through crashes with zero downtime.

### Infrastructure

- **Works with Your Runtime**: Supports v8 isolates, WebAssembly, and containers to work with your existing tools.
- **Scales to Zero**: Handle millions of connections with low latency and high-throughput writes while saving costs through instant actor sleep/wake cycles.
- **Built-in Monitoring**: Includes monitoring out of the box.
- **No Servers & No Configuration**: Deploy with one command. Scale on demand without any configuration.
- **Powered by V8 Isolates & Deno Runtime**: Faster, cheaper, and more lightweight than lambda functions & containers.
- **Supports Both Isolates & Containers**: Run your code on V8 isolates or run things like Godot/Unity game servers or video encoding in containers.
- **Simpler Than Lambda, No Timeouts Ever**: No execution time limits, no complexity — just better serverless.
- **Built with Technologies You Can Trust**: Rust, FoundationDB, the [Rivet workflow engine](docs-internal/libraries/workflow/OVERVIEW.md), and [Rivet orchestrator](packages/services/pegboard/) make Rivet delightfully boring to use.

### Use cases

- AI agents
- Game Servers
- Collaborative applications
- Local-first apps
- Discord Activities
- Chat Apps
- Yjs Sync & Storage
- Run Untrusted User Code

## Quickstart
Follow the [setup guide](https://rivet.gg/docs/setup).

### Self-hosting & manual deployment

Read our [self-hosting documentation](https://rivet.gg/docs/self-hosting).

### Rivet Cloud

Rivet Cloud is the fastest, most affordable, and most reliable way to deploy Rivet Actors with zero infrastructure maintenance. Get started at [hub.rivet.gg](https://hub.rivet.gg).

### Rivet Enterprise

Get a demo, tailored pricing to fit your needs, or have your questions answered about Rivet. Contact us [here](https://rivet.gg/sales).

## Documentation

- **Overview**
  - [Overview](https://rivet.gg/docs)

- **Getting Started**
  - [Initial Setup](https://rivet.gg/docs/setup)
  - [Actor SDK](https://jsr.io/@rivet-gg/actor/doc) (external)
  - **Client SDKs**
    - [JavaScript & TypeScript](https://jsr.io/@rivet-gg/actor-client)

- **Build with Rivet**
  - [Remote Procedure Calls](https://rivet.gg/docs/rpc)
  - [State](https://rivet.gg/docs/state)
  - [Events](https://rivet.gg/docs/events)
  - [Scaling & Concurrency](https://rivet.gg/docs/scaling)
  - [Edge Networking](https://rivet.gg/docs/edge)
  - **More**
    - [Lifecycle](https://rivet.gg/docs/lifecycle)
    - [Connections](https://rivet.gg/docs/connections)
    - [Authentication](https://rivet.gg/docs/authentication)
    - [Fault Tolerance](https://rivet.gg/docs/fault-tolerance)
    - [Logging](https://rivet.gg/docs/logging)

- **Resources**
  - [Configuration](https://rivet.gg/docs/config)
  - [Troubleshooting](https://rivet.gg/docs/troubleshooting)
  - **Self-Hosting**
    - [Overview](https://rivet.gg/docs/self-hosting)
    - [Docker Compose](https://rivet.gg/docs/self-hosting/docker-compose)
    - [Manual Deployment](https://rivet.gg/docs/self-hosting/manual-deployment)
    - [Server Config](https://rivet.gg/docs/self-hosting/server-config)
    - [Client Config](https://rivet.gg/docs/self-hosting/client-config)
  - **More**
    - [Available Regions](https://rivet.gg/docs/regions)
    - [Limitations](https://rivet.gg/docs/limitations)
    - **Advanced**
      - [Rescheduling](https://rivet.gg/docs/rescheduling)
      - [Networking](https://rivet.gg/docs/networking)
      - **Internals**
        - [Design Decisions](https://rivet.gg/docs/internals/design-decisions)
        - [Actor Runtime](https://rivet.gg/docs/internals/runtime)
- [Platform API](https://rivet.gg/docs/api)

## Community & Support

- Join our [Discord](https://rivet.gg/discord)
- Follow us on [X](https://x.com/rivet_gg)
- Follow us on [Bluesky](https://bsky.app/profile/rivet-gg.bsky.social)
- File bug reports in [GitHub Issues](https://github.com/rivet-gg/rivet/issues)
- Post questions & ideas in [GitHub Discussions](https://github.com/orgs/rivet-gg/discussions)

## Architecture

### Core technologies

- **Rust**
- **V8 & Deno**: Actor isolate runtime
- **FoundationDB**: Actor state
- **CockroachDB**: OLTP
- **ClickHouse**: Developer-facing monitoring
- **Valkey**: Caching
- **NATS**: Pub/sub
- **Traefik**: Load balancers & tunnels

### Diagram

![Architecture](./.github/media/architecture.png)

## Project layout

```
docker/                      Docker-related files
    client/                  Client image
    dev-full/                Full development environment setup
    monolith/                Monolithic Docker setup
    server/                  Server image
docs/                        Documentation
docs-internal/               Internal documentation
examples/                    Example projects
packages/                    Project packages
    api/                     API package
    common/                  Common utilities
    infra/                   Infrastructure-related code
    services/                Service implementations
    toolchain/               Toolchain-related code
resources/                   Resource files
scripts/                     Scripts for various tasks
sdks/                        SDKs
    actor/                   Actor SDK
    api/                     API SDK
```

## License

Apache 2.0
