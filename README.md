<p align="center">
  <a href="https://rivet.gg">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="./.github/media/icon-text-white.svg" alt="Rivet">
      <img src="./.github/media/icon-text-black.svg" alt="Rivet">
    </picture>
  </a>
</p>

<h3 align="center">The open-source serverless platform.</h3>
<h4 align="center">
  Easily deploy and scale AI agents, real-time applications, game servers, and complex backends on a frictionless platform that runs anywhere.
</h4>
<p align="center">
  <!-- <a href="https://github.com/rivet-gg/rivet/graphs/commit-activity"><img alt="GitHub commit activity" src="https://img.shields.io/github/commit-activity/m/rivet-gg/rivet?style=flat-square"/></a> -->
  <a href="https://github.com/rivet-gg/rivet/discussions"><img alt="GitHub Discussions" src="https://img.shields.io/github/discussions/rivet-gg/rivet?logo=github&logoColor=fff"></a>
  <a href="https://rivet.gg/discord"><img alt="Discord" src="https://img.shields.io/discord/822914074136018994?color=7389D8&label&logo=discord&logoColor=ffffff"/></a>
   <a href="https://twitter.com/rivet_gg"><img src="https://img.shields.io/twitter/follow/rivet_gg" alt="Rivet Twitter" /></a>
   <a href="https://bsky.app/profile/rivet.gg"><img src="https://img.shields.io/badge/Follow%20%40rivet.gg-4C1?color=0285FF&logo=bluesky&logoColor=ffffff" alt="Rivet Bluesky" /></a>
  <a href="/LICENSE"><img alt="License Apache-2.0" src="https://img.shields.io/github/license/rivet-gg/rivet?logo=open-source-initiative&logoColor=white"></a>
</p>

![Code snippets](./.github/media/code.png)

## Overview

Rivet is a developer-focused serverless infrastructure platform that unifies stateless functions, stateful actors, and containerized workloads. It provides simple primitives to build your backend without managing servers. Leverage Rivet Actors to create resilient, long-lived services that maintain in-memory state between requests.

Whether you’re building AI-driven services, collaborative apps, multiplayer games, or any cloud backend, Rivet’s technology provides the performance and scalability you need in a portable, open-source package.


## Key Characteristics

- **Open Source & Portable**  
  Run the Rivet platform on any infrastructure – use the fully-managed Rivet Cloud or deploy it on your own servers and cloud.

- **Unified Primitives**  
  Develop using stateless **Functions** (for request/response APIs), stateful **Actors** (for persistent, real-time services), or sandboxed **Containers** (for heavy or untrusted workloads). All are managed together or alone with a consistent CLI and tooling.

---

## Features

- **Stateful Persistence**  
  Rivet Actors preserve data in memory with automatic durability to disk. This provides persistent memory-like state – you get the speed of in-memory access with the safety of persistent storage. Ideal for dynamic, fast-moving app state (caches, game lobbies, collaborative document data, etc.).

- **Remote Procedure Calls (RPC)**  
  Lightweight built-in messaging for clients and services. Rivet offers type-safe RPC calls and broadcast events between clients and actors, simplifying real-time communication without external message brokers.

- **No Cold Starts**  
  Services hibernate on idle and wake instantly on demand. Long-running actors “sleep” when inactive and recover state immediately on the next request. Instant cold-start recovery and consistently low latency for end-users.

- **Edge Distribution**  
  Deploy backend code closer to your users. Rivet distributes actors and functions across global edge regions for ultra-low latency. Supports HTTP, WebSocket, TCP, and UDP protocols without requiring external proxies.

- **Unlimited Execution & Container Support**  
  No arbitrary execution time limits – run long-lived processes or background jobs as needed. Rivet supports anything that runs in a Docker container. If it works in Docker, it works on Rivet.

- **Fault Tolerance**  
  Actor state is persisted so that if an instance crashes or is rescheduled, it can recover its exact state with zero downtime. Combined with intelligent routing, Rivet ensures high availability.

- **Local Development**  
  Develop and test locally with ease. Spin up a full Rivet cluster with `rivet dev` or Docker Compose. Iterate locally before deploying to production.

  ### Use cases

-   AI agents
-   Multi-tenant applications
-   Local-first apps
-   Collaborative applications
-   Sandboxed Code Execution
-   Game Servers
-   Yjs Sync & Storage
-   Chat Apps

---

## Getting Started

### Quickstart (TypeScript)

_See the [full quickstart guide](https://rivet.gg/docs/actors) for a comprehensive walkthrough._

**Step 1: Create Actor**

```sh
npx create-actor@latest -p rivet -t counter
```

**Step 2: Deploy Actor**

```sh
cd your-project
npm run deploy

```

**Step 3: Monitor**

Visit the [Rivet Hub](https://hub.rivet.gg) to create & test your actors.

### Documentation

-   [**Documentation**](https://rivet.gg/docs): Read about how to use Rivet
-   [**Examples**](./examples/): See existing Rivet projects
-   [**Contributing**](./CONTRIBUTING.md): Learn to contribute to Rivet

### Running Rivet

-   **Self-Hosting**
    -   [**Local Dev & Single Container**](https://rivet.gg/docs/self-hosting/single-container): Great for local development, fast single-node deployments, and testing Rivet
    -   [**Docker Compose**](https://rivet.gg/docs/self-hosting/docker-compose): Great for hobbyist & single-node deployments
    -   [**Manual**](https://rivet.gg/docs/self-hosting/manual-deployment): Run on your own VMs without Docker
-   [**Rivet Cloud**](https://hub.rivet.gg): Fastest, most affordable, and most reliable way to deploy Rivet Actors with zero infrastructure maintenance
-   [**Rivet Enterprise**](https://rivet.gg/sales): Get a demo or have your questions answered about Rivet

## Community & Support

-   Join our [**Discord**](https://rivet.gg/discord)
-   Follow us on [**X**](https://x.com/rivet_gg)
-   Follow us on [**Bluesky**](https://bsky.app/profile/rivet-gg.bsky.social)
-   File bug reports in [**GitHub Issues**](https://github.com/rivet-gg/rivet/issues)
-   Post questions & ideas in [**GitHub Discussions**](https://github.com/rivet-gg/rivet/discussions)

## Technologies

-   **Rust**
-   **V8 & Deno**: Actor isolate runtime
-   **FoundationDB**: Actor state
-   **CockroachDB**: OLTP
-   **ClickHouse**: Developer-facing monitoring
-   **Valkey**: Caching
-   **NATS**: Pub/sub
-   **Traefik**: Load balancers & tunnels

<!-- ### Diagram

![Architecture](./.github/media/architecture.png) -->

## Project layout

```
docker/                      Docker-related files
    dev-full/                Full development environment setup
    monolith/                Monolithic Docker setup
    universal/               Universal multi-stage builder image
docs/                        Documentation
docs-internal/               Internal documentation
examples/                    Example projects
frontend/                    Rivet Hub & other frontend components
packages/                    Project packages
    api/                     API package
    common/                  Common utilities
    infra/                   Infrastructure-related code
    services/                Service implementations
    toolchain/               Toolchain-related code
resources/                   Misc resources supporting Rivet
scripts/                     Scripts for various tasks
sdks/                        SDKs
    actor/                   Actor SDK
    api/                     Low-level SDK for calling API
site/                        Website & documentation
```

## License

Apache 2.0
