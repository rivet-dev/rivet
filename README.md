<p align="center">
  <picture>
      <source media="(prefers-color-scheme: dark)" srcset="./media/icon-text-white.svg" alt="Rivet">
      <img src="./media/icon-text-black.svg" alt="Rivet">
  </picture>
</p>

<p align="center">
  <a href="/docs/philosophy/LICENSING.md"><img alt="License Apache-2.0" src="https://img.shields.io/github/license/rivet-gg/rivet?style=flat-square"></a>
  <a href="https://github.com/rivet-gg/rivet/graphs/commit-activity"><img alt="GitHub commit activity" src="https://img.shields.io/github/commit-activity/m/rivet-gg/rivet?style=flat-square"/></a>
  <a href="https://github.com/rivet-gg/rivet/issues"><img alt="GitHub closed issues" src="https://img.shields.io/github/issues-closed/rivet-gg/rivet?style=flat-square"/></a>
  <a href="https://rivet.gg/discord"><img alt="GitHub closed issues" src="https://img.shields.io/discord/822914074136018994?style=flat-square&label=discord"/></a>
</p>

## Features

### 🎮 Dedicated Game Servers

Deploy game servers in minutes across multiple regions & clouds providers.

-   Optimize for performance or cost, servers <Tooltip tip="Shared CPU core similar to VPS providers, see pricing page for details">starting at $2.85/mo</Tooltip>
-   Auto-scales 90% faster than AWS GameLift
-   No-downtime deploys & instant rollbacks
-   Monitoring & crash reporting

[Documentation](https://rivet.gg/docs/dynamic-servers)

### 🛡️ DDoS Mitigation

-   No added latency
-   Supports UDP & TCP & WebSockets & WebRTC
-   Automatic SSL for game servers (WebSockets & TCP+TLS)

[Documentation](https://rivet.gg/docs/dynamic-servers/concepts/game-guard)

### 🌐 CDN

Asset delivery, game downloads, & website hosting

-   Custom domains
-   Instant rollbacks
-   Automatic SSL

[Documentation](https://rivet.gg/docs/cdn)

### 🧩 Backend Modules

Write server-side logic using TypeScript (or use your own API server)

-   Modules include matchmaking, parties, authentication, & more
-   Postgres database included for persistence
-   Powered by <a href="https://github.com/rivet-gg/opengb">Open Game Backend</a>

[Documentation](https://opengb.dev/)

## 🚀 Getting Started

**Self-hosting & development**

See the [setup guide](/docs/getting_started/DEVELOPMENT.md) to develop & deploy Rivet yourself.

**Rivet Cloud**

[Rivet Cloud](https://rivet.gg) is the fastest and most affordable way to get your game up and running. Sign up at [rivet.gg](https://rivet.gg).

## 💬 Community & Support

**Discord**

[Invite](https://rivet.gg/discord)

-   **Lounge** The Rivet team is remote and does most of their work in public Discord voice chat. Come drop by if you have questions or want to hang!
-   **#support** Ask questions about getting your game runnin on Rivet
-   **#open-source-dev** Ask questions about the open source repo
-   **#showcase** Show off your game, get feedback

**Releases**

Stay up to date on the latest releases on [X](https://x.com/rivet_gg).

Technical release notes can be subscribed to by watcing this repository.

**Bugs & Feature Requests**

Bugs and feature requests can be submitted as a GitHub Issue.

**Roadmap**

We create public issues for most items on our roadmpa.

Subscribe to issues to get notified when they're updated. Add a 👍 reaction to issues to get them prioritized faster

## 📐 Architecture

We maintain a detailed architecture diagram [here](https://www.figma.com/file/GvCj77EG79NUoW1dRG4qkg/Architecture?type=whiteboard&node-id=0%3A1&t=WqMQ2r6avjM0jPK0-1).

![Architecture](./media/architecture.png)

## 📖 Documentation

### Game developers

Visit our documentation for game developers [here](https://rivet.gg/docs).

### Internal documentation

<!--

GPT prompt:

Convert this to a markdown list with indents with links to the document and a human readable name:

$(tree docs/)

-->

-   [About](docs/about)
    -   [Telemetry](docs/about/TELEMETRY.md)
-   [Getting Started](docs/getting_started)
    -   [Debugging](docs/getting_started/DEBUGGING.md)
    -   [Development Firewalls](docs/getting_started/DEVELOPMENT_FIREWALLS.md)
    -   [Development](docs/getting_started/DEVELOPMENT.md)
    -   [Project Structure](docs/getting_started/PROJECT_STRUCTURE.md)
    -   [Rust Analyzer](docs/getting_started/RUST_ANALYZER.md)
    -   [Services](docs/getting_started/SERVICES.md)
-   [Infrastructure](docs/infrastructure)
    -   [Alertmanager](docs/infrastructure/alertmanager/TESTING_ALERTS.md)
    -   [ClickHouse](docs/infrastructure/clickhouse)
        -   [Readme](docs/infrastructure/clickhouse/README.md)
        -   [Troubleshooting](docs/infrastructure/clickhouse/TROUBLESHOOTING.md)
        -   [Why ClickHouse](docs/infrastructure/clickhouse/WHY_CLICKHOUSE.md)
    -   [Cockroach](docs/infrastructure/cockroach)
        -   [Readme](docs/infrastructure/cockroach/README.md)
        -   [Why Cockroach](docs/infrastructure/cockroach/WHY_COCKRAOCH.md)
    -   [Helm](docs/infrastructure/helm/TROUBLESHOOTING.md)
    -   [Imagor](docs/infrastructure/imagor/MEDIA_DELIVERY_AND_RESIZING.md)
    -   [K3d](docs/infrastructure/k3d/TROUBLESHOOTING.md)
    -   [K8s](docs/infrastructure/k8s)
        -   [Tips](docs/infrastructure/k8s/TIPS.md)
        -   [Troubleshooting](docs/infrastructure/k8s/TROUBLESHOOTING.md)
    -   [Minio](docs/infrastructure/minio/TROUBLESHOOTING.md)
    -   [Nats](docs/infrastructure/nats/TROUBLESHOOTING.md)
    -   [Networking](docs/infrastructure/networking)
        -   [Edge Cluster Networking](docs/infrastructure/networking/EDGE_CLUSTER_NETWORKING.md)
        -   [IP Ranges](docs/infrastructure/networking/IP_RANGES.md)
        -   [IPv6](docs/infrastructure/networking/IPV6.md)
    -   [Nix](docs/infrastructure/nix)
        -   [Lorri](docs/infrastructure/nix/LORRI.md)
        -   [Readme](docs/infrastructure/nix/README.md)
    -   [Nomad](docs/infrastructure/nomad/README.md)
    -   [Prometheus](docs/infrastructure/prometheus/README.md)
    -   [Protobuf](docs/infrastructure/protobuf/TIMESTAMPS.md)
    -   [Redis](docs/infrastructure/redis)
        -   [Hosting Providers](docs/infrastructure/redis/HOSTING_PROVIDERS.md)
        -   [Readme](docs/infrastructure/redis/README.md)
        -   [Sharding](docs/infrastructure/redis/SHARDING.md)
        -   [Tips](docs/infrastructure/redis/TIPS.md)
        -   [Troubleshooting](docs/infrastructure/redis/TROUBLESHOOTING.md)
        -   [Why Redis](docs/infrastructure/redis/WHY_REDIS.md)
    -   [Rust](docs/infrastructure/rust/TROUBLESHOOTING.md)
    -   [S3](docs/infrastructure/s3/TROUBLESHOOTING.md)
    -   [SBOM](docs/infrastructure/SBOM.md)
    -   [Terraform](docs/infrastructure/terraform)
        -   [Configs and Secrets](docs/infrastructure/terraform/CONFIGS_AND_SECRETS.md)
        -   [Readme](docs/infrastructure/terraform/README.md)
        -   [Troubleshooting](docs/infrastructure/terraform/TROUBLESHOOTING.md)
    -   [Timeouts](docs/infrastructure/TIMEOUTS.md)
    -   [Traefik](docs/infrastructure/traefik)
        -   [Ing Job Sizing Methodology](docs/infrastructure/traefik/ING_JOB_SIZING_METHODOLOGY.md)
        -   [Readme](docs/infrastructure/traefik/README.md)
        -   [Router Priorities](docs/infrastructure/traefik/ROUTER_PRIORITIES.md)
    -   [Traffic Server](docs/infrastructure/traffic_server)
        -   [Readme](docs/infrastructure/traffic_server/README.md)
        -   [Why Traffic Server](docs/infrastructure/traffic_server/WHY_TRAFFIC_SERVER.md)
-   [Libraries](docs/libraries)
    -   [Bolt](docs/libraries/bolt)
        -   [Buggy Cache Purging](docs/libraries/bolt/BUGGY_CACHE_PURGING.md)
        -   [Config](docs/libraries/bolt/config)
            -   [Cloudflare](docs/libraries/bolt/config/CLOUDFLARE.md)
            -   [DNS](docs/libraries/bolt/config/DNS.md)
            -   [Linode](docs/libraries/bolt/config/LINODE.md)
            -   [Namespace](docs/libraries/bolt/config/NAMESPACE.md)
            -   [Sendgrid](docs/libraries/bolt/config/SENDGRID.md)
        -   [Debugging Services](docs/libraries/bolt/DEBUGGING_SERVICES.md)
        -   [Feature Flagging](docs/libraries/bolt/FEATURE_FLAGGING.md)
        -   [Readme](docs/libraries/bolt/README.md)
        -   [Regions](docs/libraries/bolt/REGIONS.md)
    -   [Chirp](docs/libraries/chirp)
        -   [Error Handling](docs/libraries/chirp/ERROR_HANDLING.md)
        -   [Glossary](docs/libraries/chirp/GLOSSARY.md)
        -   [Readme](docs/libraries/chirp/README.md)
    -   [Claims](docs/libraries/claims/JWT.md)
-   [Packages](docs/packages)
    -   [Api-Auth](docs/packages/api-auth/HUB_AUTH.md)
    -   [Cluster](docs/packages/cluster)
        -   [Autoscaling](docs/packages/cluster/AUTOSCALING.md)
        -   [Server Provisioning](docs/packages/cluster/SERVER_PROVISIONING.md)
        -   [TLS and DNS](docs/packages/cluster/TLS_AND_DNS.md)
    -   [Job](docs/packages/job/DOCKER_IMAGE_DELIVERY.md)
    -   [Mm](docs/packages/mm/IDLE_LOBBIES.md)
    -   [Upload](docs/packages/upload/UPLOADS.md)
-   [Philosophy](docs/philosophy)
    -   [Infra as Code](docs/philosophy/INFRA_AS_CODE.md)
    -   [Licensing](docs/philosophy/LICENSING.md)
    -   [Why Open Source](docs/philosophy/WHY_OPEN_SOURCE.md)
-   [Processes](docs/processes)
    -   [Changelog](docs/processes/CHANGELOG.md)
    -   [Deploy Process](docs/processes/DEPLOY_PROCESS.md)
    -   [Making Changes](docs/processes/MAKING_CHANGES.md)
    -   [Project Management](docs/processes/PROJECT_MANAGEMENT.md)
    -   [Pull Requests](docs/processes/PULL_REQUESTS.md)
    -   [Refactoring](docs/processes/REFACTORING.md)
    -   [Releasing](docs/processes/RELEASING.md)
    -   [Versioning](docs/processes/VERSIONING.md)

<!--
## We're hiring!

We're a team of scrappy engineers willing to get our hands dirty with everything from Linux internals, niche game engines, designs that don't look like [this](https://www.linears.art/), and god-tier developer experiences. If you prefer reading the source instead of documentation, love hacking on games in your free time, and have a healthy dose of anarchy in you, come [join us!](https://rivet-gg.notion.site/Job-Board-eed66f2eab2b4d7ea3e21ccd63b22efe?pvs=4)
-->

## License

Apache 2.0

_Trust no-one, own your backend_
