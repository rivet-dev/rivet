<div align="center">
  <a href="https://rivet.gg">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="./.github/media/icon-text-white.svg" alt="Rivet">
      <img src="./.github/media/icon-text-black.svg" alt="Rivet" height="75">
    </picture>
  </a>
  <br/>
  <br/>
  <p><b>Scale and orchestrate <a href="https://github.com/rivet-gg/rivetkit">RivetKit</a> workloads</b></p>
  <p>
    Built to provide production grade scale and orchestration for the most intensive workloads.
  </p>
  <p>
    <a href="https://rivet.gg/docs/actors/quickstart">Quickstart</a> •
    <a href="https://rivet.gg/docs/actors">Documentation</a> •
    <a href="https://rivet.gg/docs/general/self-hosting">Self-Hosting</a> •
    <a href="https://rivet.gg/discord">Discord</a> •
    <a href="https://x.com/rivet_gg">X</a> •
    <a href="https://bsky.app/profile/rivet.gg">Bluesky</a>
  </p>
  <!--<p>
    <a href="https://github.com/rivet-gg/rivet/discussions"><img alt="GitHub Discussions" src="https://img.shields.io/github/discussions/rivet-gg/rivet?logo=github&logoColor=fff"></a>
    <a href="https://rivet.gg/discord"><img alt="Discord" src="https://img.shields.io/discord/822914074136018994?color=7389D8&label&logo=discord&logoColor=ffffff"/></a>
    <a href="https://twitter.com/rivet_gg"><img src="https://img.shields.io/twitter/follow/rivet_gg" alt="Rivet Twitter" /></a>
    <a href="https://bsky.app/profile/rivet.gg"><img src="https://img.shields.io/badge/Follow%20%40rivet.gg-4C1?color=0285FF&logo=bluesky&logoColor=ffffff" alt="Rivet Bluesky" /></a>
    <a href="/LICENSE"><img alt="License Apache-2.0" src="https://img.shields.io/github/license/rivet-gg/rivet?logo=open-source-initiative&logoColor=white"></a>
  </p>-->
</div>

## Projects

Public-facing projects:

- **Rivet Engine** (you are here): Engine that powers RivetKit at scale
- **[RivetKit](https://github.com/rivet-gg/rivetkit)**: Lightweight TypeScript library for building Rivet Actors — works with Redis or Rivet Engine
- **[Rivet Inspector](/frontend/apps/studio)**: Like Postman, but for Rivet Actors
- **[Rivet Hub](/frontend/apps/hub)**: UI for Rivet Engine
- **[Rivet Documentation](/site/src/content/docs)**

Projects powering Rivet Engine:

- **[Pegboard](packages/services/pegboard/)**: Actor orchestrator
- **[Guard](packages/core/guard/)**: Proxy for routing traffic to Rivet Actors
- **[Gasoline](packages/common/gasoline/)**: Core durable execution engine that powers Rivet

## Get Started

__QUICKSTART__

__FEATURES__

## Examples

__EXAMPLES__

## Running Rivet

The ability to self-host Rivet Engine is currently currently a work in progress.

Please see the self-hosting guide for [RivetKit](https://www.rivet.gg/docs/general/self-hosting/).

For enterprise use cases, [get in touch](https://rivet.gg/sales) about using Rivet Cloud or self-hosting.

__COMMUNITY__

## Technologies

-   **Rust**
-   **FoundationDB**: State
-   **NATS**: Pub/sub
-   **Valkey**: Caching
-   **ClickHouse**: Monitoring

__LICENSE__

