<p align="center">
  <a href="https://rivet.gg">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="./.github/media/icon-text-white.svg" alt="Rivet">
      <img src="./.github/media/icon-text-black.svg" alt="Rivet">
    </picture>
  </a>
</p>

<h3 align="center">Scalable. Stateful. Serverless.</h3>
<h4 align="center">
  Rivet is the platform to build realtime, edge, or agent applications.<br/>
  No limitations of Redis or timeouts of Lambda.
</h4>
<p align="center">
  <!-- <a href="https://github.com/rivet-gg/rivet/graphs/commit-activity"><img alt="GitHub commit activity" src="https://img.shields.io/github/commit-activity/m/rivet-gg/rivet?style=flat-square"/></a> -->
  <a href="https://github.com/orgs/rivet-gg/discussions"><img alt="GitHub Discussions" src="https://img.shields.io/github/discussions/rivet-gg/rivet?logo=github&logoColor=fff"></a>
  <a href="https://rivet.gg/discord"><img alt="Discord" src="https://img.shields.io/discord/822914074136018994?color=7389D8&label&logo=discord&logoColor=ffffff"/></a>
   <a href="https://twitter.com/rivet_gg"><img src="https://img.shields.io/twitter/follow/rivet_gg" alt="Rivet Twitter" /></a>
   <a href="https://bsky.app/profile/rivet.gg"><img src="https://img.shields.io/badge/Follow%20%40rivet.gg-4C1?color=0285FF&logo=bluesky&logoColor=ffffff" alt="Rivet Bluesky" /></a>
  <a href="/LICENSE"><img alt="License Apache-2.0" src="https://img.shields.io/github/license/rivet-gg/rivet?logo=open-source-initiative&logoColor=white"></a>
</p>

![Code snippets](./.github/media/code.png)

## Intro

Rivet comes with simple primitives to build your backend. Leverage Rivet Actors to build complex functionality with ease.

### Features

-   [**State & Persistence**](https://rivet.gg/docs/state): State that feels like memory but works like storage. Ideal for dynamic, fast-moving apps.
-   [**Remote Procedure Calls**](https://rivet.gg/docs/rpc): Lightweight messaging built for speed. Complete client/server type safety included.
-   [**Runs Forever, Sleeps When Idle**](https://rivet.gg/docs/lifecycle): Always available, sleeps on network inactivity or timeouts, and wakes instantly on demand.
-   [**Edge Networking**](https://rivet.gg/docs/edge): Automatically distribute your applications near your users for ultra-low latency.
-   [**Fault Tolerance**](https://rivet.gg/docs/fault-tolerance): Ensure application & state resilience through crashes with zero downtime.

### Infrastructure

-   **Works with Your Runtime**: Supports V8 isolates, WebAssembly, and containers to work with your existing tools.
-   **Scales to Zero**: Handle millions of connections with low latency and high-throughput writes while saving costs through instant actor sleep/wake cycles.
-   **No Servers & No Configuration**: Deploy with one command. Scale on demand without any configuration.
-   **Simpler Than Lambda, No Timeouts Ever**: No execution time limits, no complexity — just better serverless.
-   **Batteries Included Monitoring**: Includes monitoring out of the box, or bring your own monitoring.
-   **Built with Technologies You Can Trust**: Rust, FoundationDB, the [Rivet workflow engine](docs-internal/libraries/workflow/OVERVIEW.md), and [Rivet orchestrator](packages/services/pegboard/) make Rivet delightfully boring to use.

### Use cases

-   AI agents
-   Game Servers
-   Collaborative applications
-   Local-first apps
-   Discord Activities
-   Chat Apps
-   Yjs Sync & Storage
-   Sandboxed Code Execution

## Getting Started

### Quickstart

_See the [full quickstart guide](https://rivet.gg/docs/setup) for a comprehensive walkthrough._

**Step 1: Install CLI**

```sh
# macOS & Linux & WSL
curl -fsSL https://releases.rivet.gg/rivet/latest/install.sh | sh

# Windows (cmd)
powershell -Command "iwr https://releases.rivet.gg/rivet/latest/install.ps1 -useb | iex"

# Windows (PowerShell)
iwr https://releases.rivet.gg/rivet/latest/install.ps1 -useb | iex
```

**Step 2: Create Project & Deploy**

```sh
rivet init
cd my-app
rivet deploy
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
-   Post questions & ideas in [**GitHub Discussions**](https://github.com/orgs/rivet-gg/discussions)

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
