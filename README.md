<!-- 
THIS FILE IS AUTO-GENERATED. DO NOT EDIT DIRECTLY.
To update this README, run: npm run gen:readme
Generated from: site/scripts/generateReadme.mjs
-->

<div align="center">
  <a href="https://rivet.gg">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="./.github/media/icon-text-white.svg" alt="Rivet">
      <img src="./.github/media/icon-text-black.svg" alt="Rivet" height="75">
    </picture>
  </a>
  <br/>
  <br/>
  <p><b>Scale and orchestrate RivetKit workloads</b></p>
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
- **[Chirp](packages/common/chirp-workflow/)**: Core workflow engine that powers Rivet

## Get Started

Get started with Rivet by following a quickstart guide:

- [Node.js & Bun](https://www.rivet.gg/docs/actors/quickstart/backend/)
- [React](https://www.rivet.gg/docs/actors/quickstart/react/)


## Features

Rivet Actors, a primitive of RivetKit, provide everything you need to build fast, scalable, and real-time applications without the complexity. Rivet Engine is the core of self-hosting and is used for orchestrating actors at scale.

- **Long-Lived, Stateful Compute**: Like AWS Lambda but with memory and no timeouts
- **Blazing-Fast Reads & Writes**: State stored on same machine as compute  
- **Realtime, Made Simple**: Built-in WebSockets and SSE support
- **Store Data Near Your Users**: Deploy to the edge for low-latency access
- **Infinitely Scalable**: Auto-scale from zero to millions without configuration
- **Fault Tolerant**: Automatic error handling and recovery built-in

## BYO DB (Bring Your Own Database)
The Rivet Engine supports:

- **PostgreSQL**: For production deployments
- **FoundationDB**: For enterprise-scale distributed systems
- **Filesystem**: For single-node deployments

## Examples

- AI Agent — [GitHub](https://github.com/rivet-gg/rivetkit/tree/main/examples/ai-agent) · [StackBlitz](https://stackblitz.com/github/rivet-gg/rivetkit/tree/main/examples/ai-agent)
- Chat Room — [GitHub](https://github.com/rivet-gg/rivetkit/tree/main/examples/chat-room) · [StackBlitz](https://stackblitz.com/github/rivet-gg/rivetkit/tree/main/examples/chat-room)
- Collab (Yjs) — [GitHub](https://github.com/rivet-gg/rivetkit/tree/main/examples/crdt) · [StackBlitz](https://stackblitz.com/github/rivet-gg/rivetkit/tree/main/examples/crdt)
- Multiplayer Game — [GitHub](https://github.com/rivet-gg/rivetkit/tree/main/examples/game) · [StackBlitz](https://stackblitz.com/github/rivet-gg/rivetkit/tree/main/examples/game)
- Local-First Sync — [GitHub](https://github.com/rivet-gg/rivetkit/tree/main/examples/sync) · [StackBlitz](https://stackblitz.com/github/rivet-gg/rivetkit/tree/main/examples/sync)
- Rate Limiter — [GitHub](https://github.com/rivet-gg/rivetkit/tree/main/examples/rate) · [StackBlitz](https://stackblitz.com/github/rivet-gg/rivetkit/tree/main/examples/rate)
- Per-User DB — [GitHub](https://github.com/rivet-gg/rivetkit/tree/main/examples/database) · [StackBlitz](https://stackblitz.com/github/rivet-gg/rivetkit/tree/main/examples/database)
- Multi-Tenant SaaS — [GitHub](https://github.com/rivet-gg/rivetkit/tree/main/examples/tenant) · [StackBlitz](https://stackblitz.com/github/rivet-gg/rivetkit/tree/main/examples/tenant)
- Stream Processing — [GitHub](https://github.com/rivet-gg/rivetkit/tree/main/examples/stream) · [StackBlitz](https://stackblitz.com/github/rivet-gg/rivetkit/tree/main/examples/stream)

## Running Rivet

The ability to self-host Rivet Engine is currently currently a work in progress.

Please see the self-hosting guide for [RivetKit](https://www.rivet.gg/docs/general/self-hosting/).

For enterprise use cases, [get in touch](https://rivet.gg/sales) about using Rivet Cloud or self-hosting.

## Community & Support

Join thousands of developers building with Rivet Actors today:

- [Discord](https://rivet.gg/discord) - Chat with the community
- [X/Twitter](https://x.com/rivet_gg) - Follow for updates
- [Bluesky](https://bsky.app/profile/rivet.gg) - Follow for updates
- [GitHub Discussions](https://github.com/rivet-gg/rivetkit/discussions) - Ask questions and share ideas
- [GitHub Issues](https://github.com/rivet-gg/rivetkit/issues) - Report bugs and request features
- [Talk to an engineer](https://rivet.gg/talk-to-an-engineer) - Discuss your technical needs, current stack, and how Rivet can help with your infrastructure challenges

## Technologies

-   **Rust**
-   **FoundationDB**: State
-   **NATS**: Pub/sub
-   **Valkey**: Caching
-   **ClickHouse**: Monitoring

## License

[Apache 2.0](LICENSE)

