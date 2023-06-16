<p align="center">
  <img alt="rivet_logo" src="./media/banner.png">
</p>

<p align="center">
  <i>Managed game servers, matchmaking, and DDoS mitigation that lets you focus on building your game.</i>
</p>

<p align="center">
  <img alt="GitHub" src="https://img.shields.io/github/license/rivet-gg/rivet?style=flat-square">
  <a href='http://makeapullrequest.com'><img alt='PRs Welcome' src='https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square'/></a>
  <img alt="GitHub commit activity" src="https://img.shields.io/github/commit-activity/m/rivet/rivet?style=flat-square"/>
  <img alt="GitHub closed issues" src="https://img.shields.io/github/issues-closed/rivet/rivet?style=flat-square"/>
</p>

<p align="center">
  <a href="https://rivet.gg/">Home</a> - <a href="https://docs.rivet.gg/">Docs</a> - <a href="https://twitter.com/rivet_gg">Twitter</a> - <a href="https://discord.gg/BG2vqsJczH">Discord</a>
</p>


## 👾 Features

- Everything is accessible from an easy to use [GUI, CLI, or API](https://docs.rivet.gg/general/gui-cli-api)
- [Serverless Lobbies](https://docs.rivet.gg/serverless-lobbies/introduction) for auto-scaling game lobbies
- [Flexible matchmaker](https://docs.rivet.gg/matchmaker/introduction) for placing players in lobbies with no wait times
- [CDN](https://docs.rivet.gg/cdn/introduction) for hosting assets and webpages with a custom domain or provided rivet.game subdomain
- [Game Guard](https://docs.rivet.gg/serverless-lobbies/concepts/game-guard) for DDoS mitigation and managed WebSocket SSL & TCP+TLS termination
- Streamlined DevOps for teams
- Unified logging & monitoring & analytics
- No downtime deploys with easy rollbacks

<p align="center">
  <img alt="rivet_screenshot" src="./media/splash_screenshot.png">
</p>

## 🚀 Getting Started

### Rivet Cloud

[Rivet Cloud](https://rivet.gg) is the fastest way to get your game up and running. Sign up at [rivet.gg](https://rivet.gg) and get a free game server.

### Self-hosting

See the [setup guide](./doc/SETUP.md) to develop & deploy Rivet yourself.

## 📐 Architecture

Below is a simplified architecture diagram of a Rivet cluster.

![Architecture](./media/simplified_architecture.png)

## 📖 Helpful Docs

> **Looking for documentation on building a game with Rivet?**
> 
> Visit our [documentation for game developers](https://docs.rivet.gg/)!

**Core Components**

-   [Project Structure](/doc/PROJECT_STRUCTURE.md)
-   [Bolt](/doc/bolt/README.md)
-   [Chirp](/doc/chirp/README.md)

**Operating a Rivet Cluster**

-   [Developing Services](/doc/DEVELOPING_SERVICES.md)
-   [Working with Databases](/doc/WORKING_WITH_DATABASES.md)
-   [Terraform](/doc/tf/README.md)
-   [SaltStack](/doc/saltstack/README.md)

**3rd Party Services**

-   Databases
    -   [Cockroach](/doc/cockroach/README.md)
    -   [ClickHouse](/doc/clickhouse/README.md)
    -   [Redis](/doc/redis/README.md)
-   Infrastructure
    -   [Nomad](/doc/nomad/README.md)
    -   [Consul](/doc/consul/README.md)
    -   [Traefik](/doc/traefik/README.md)
    -   [Nebula](/doc/nebula/README.md)
-   Development
    -   [Nix](/doc/nix/README.md)

**Writing services**

-   [Error Handling](/doc/ERROR_HANDLING.md)
