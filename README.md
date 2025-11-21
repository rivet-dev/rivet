<div align="center">
  <a href="https://www.rivet.dev">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="./.github/media/logo/icon-text-white.svg" alt="Rivet">
      <img src="./.github/media/logo/icon-text-black.svg" alt="Rivet" height="75">
    </picture>
  </a>
  <br/>
  <br/>
  <p><b>Build and scale stateful workloads</b></p>
  <p>
    Rivet is an open-source library for long-lived processes.<br/>
    Like Lambda — but with realtime, persistence, and hibernation.<br/>
    Easily <b>self-hostable</b> and works <b>with your infrastructure</b>.
  </p>
  <p>
    <a href="https://www.rivet.dev/docs/actors/quickstart">Quickstart</a> •
    <a href="https://www.rivet.dev/docs/actors">Documentation</a> •
    <a href="https://www.rivet.dev/changelog">Changelog</a> •
    <a href="https://www.rivet.dev/discord">Discord</a> •
    <a href="https://x.com/rivet_dev">X</a>
  </p>
</div>

## Projects

Projects that live in this repository:

- **RivetKit**: Library for building with Rivet
  - **[TypeScript](/rivetkit-typescript)**: Client & server
  - **[Rust](/rivetkit-typescript)**, Client only, experimental
  - **[Python](/rivetkit-typescript)**: Client only, experimental
  - **[OpenAPI](/rivetkit-openapi)**: OpenAPI spec for RivetKit
- **[Rivet Engine](/engine)**: High-performance, Rust-based engine that powers Rivet at scale
  - **[Pegboard](packages/services/pegboard/)**: Actor orchestrator & networking
  - **[Gasoline](/engine/packages/gasoline)**: Durable execution engine that powers Rivet
  - **[Guard](/engine/packages/guard)**: Proxy for routing traffic to Rivet Actors
  - **[Epoxy](/engine/packages/epoxy)**: Multi-region KV store based on Epaxos
- **[Rivet Dashboard](/frontend/apps/studio)**: Like Chrome Inspector, but for Rivet
- **[Rivet Documentation](/website/src/content/docs)**: Documentation at [rivet.dev/docs](https://www.rivet.dev)
- **[Rivet Website](/website)**: Source code for [rivet.dev](https://www.rivet.dev)

## Features

Rivet provides everything you need to build fast, scalable, and real-time applications without the complexity.

- **Long-Lived, Stateful Compute**: Persistent memory between requests
- **No Database Round Trips**: State stored locally for speed
- **Realtime, Made Simple**: WebSocket/SSE updates built-in
- **Sleep When Idle**: Hibernation with zero cold starts
- **Insane Scale**: Auto-scale from zero to millions
- **Resilient by Design**: Built-in failover and recovery
- **Edge State Storage**: Geographically distributed state
- **Web Standards**: WebSockets, SSE, HTTP compatible
- **No Vendor Lock-In**: Open-source, self-hostable

## Examples

- [AI Agent](/examples/ai-agent)
- [Realtime Docs](/examples/crdt)
- [Workflows](/examples/workflows)
- [Local-First Sync](/examples/sync)
- [Per-User Database](/examples/database)
- [Background Jobs](/examples/background-jobs)
- [Rate Limiting](/examples/rate)
- [Multiplayer Game](/examples/game)
- [Bots](/examples/bots)

## Getting Started

### Building with Rivet

Get started with Rivet by following a quickstart guide:

- **[Node.js & Bun Quickstart](https://www.rivet.dev/docs/actors/quickstart/backend)** - Set up Rivet with Node.js, Bun, and web frameworks
- **[React Quickstart](https://www.rivet.dev/docs/actors/quickstart/react)** - Set up Rivet with React
- **[Next.js Quickstart](https://www.rivet.dev/docs/actors/quickstart/next-js)** - Set up Rivet with Next.js
- **[Cloudflare Workers Quickstart](https://www.rivet.dev/docs/actors/quickstart/cloudflare-workers)** - Set up Rivet with Cloudflare Workers

### Rivet Cloud

The fastest way to deploy Rivet is to use [**Rivet Cloud**](https://dashboard.rivet.dev) with [**your cloud of choice**](https://www.rivet.dev/docs/#deploy-options). Rivet Cloud provides multiple edge regions & cloud providers for you.

### Self-Hosting

_You do not need Rivet Engine to get started with Rivet._

View more options & deployment guides in the [**self-hosting documentation**](https://www.rivet.dev/docs/self-hosting/).

```bash
# Docker
docker run -p 6420:6420 rivetkit/engine

# macOS (Apple Silicon)
curl -o rivet-engine "https://releases.rivet.dev/rivet/latest/rivet-engine-aarch64-apple-darwin" && chmod +x rivet-engine && ./rivet-engine start

# macOS (Intel)
curl -o rivet-engine "https://releases.rivet.dev/rivet/latest/rivet-engine-x86_64-apple-darwin" && chmod +x rivet-engine && ./rivet-engine start

# Linux (x86)
curl -o rivet-engine "https://releases.rivet.dev/rivet/latest/rivet-engine-x86_64-unknown-linux-musl" && chmod +x rivet-engine && ./rivet-engine start

# Windows (x86_64)
curl.exe -o rivet-engine.exe "https://releases.rivet.dev/rivet/latest/rivet-engine-x86_64-pc-windows-gnu.exe"
.\rivet-engine.exe start
```

## Runs Anywhere

Deploy Rivet Actors anywhere - from serverless platforms to your own infrastructure with Rivet's flexible runtime options.

### Storage
- [Rivet Cloud](https://www.rivet.dev/docs/cloud) *(1-Click Deploy)*
- [Postgres](https://www.rivet.dev/docs/actors/)
- [File System](https://www.rivet.dev/docs/actors/quickstart/backend)
- [Memory](https://www.rivet.dev/docs/actors/quickstart/backend)

### Compute
- [Node.js](https://www.rivet.dev/docs/actors/quickstart/backend)
- [Bun](https://www.rivet.dev/docs/actors/quickstart/backend)
- [Deno](https://github.com/rivet-dev/rivetkit/tree/9a3d850aee45167eadf249fdbae60129bf37e818/examples/deno)
- [Vercel](https://www.rivet.dev/docs/connect/vercel) *(1-Click Deploy)*
- [Railway](https://railway.com/deploy/rivet) *(1-Click Deploy)*
- [Durable Objects](https://www.rivet.dev/docs/actors/quickstart/backend)
- [Kubernetes](https://www.rivet.dev/docs/connect/kubernetes)
- [AWS ECS](https://www.rivet.dev/docs/connect/aws-ecs)
- [Google Cloud Run](https://www.rivet.dev/docs/connect/gcp-cloud-run)
- [Hetzner](https://www.rivet.dev/docs/connect/hetzner)
- [VM & Bare Metal](https://www.rivet.dev/docs/connect/vm-and-bare-metal)
- [AWS Lambda](https://www.rivet.dev/docs/connect/aws-lambda) *(On The Roadmap)*
- [Supabase](https://www.rivet.dev/docs/connect/supabase) *(On The Roadmap)*
- [Freestyle](https://www.rivet.dev/docs/connect/freestyle) *(On The Roadmap)*

## Works With Your Tools

Seamlessly integrate Rivet with your favorite frameworks, languages, and tools. Don't see what you need? [Request an integration](https://github.com/rivet-dev/rivetkit/issues/new).

### Frontend & Clients
- [JavaScript](https://www.rivet.dev/docs/clients/javascript)
- [TypeScript](https://www.rivet.dev/docs/clients/javascript)
- [React](https://www.rivet.dev/docs/clients/react)
- [Rust](https://www.rivet.dev/docs/clients/rust)
- [Next.js](https://www.rivet.dev/docs/clients/next-js)
- [Svelte](https://github.com/rivet-dev/rivetkit/pull/1172)
- [Vue](https://github.com/rivet-dev/rivetkit/issues/903) *(Help Wanted)*

### Backend
- [Hono](https://www.rivet.dev/docs/integrations/hono)
- [Express](https://www.rivet.dev/docs/integrations/express)
- [Elysia](https://www.rivet.dev/docs/integrations/elysia)
- [tRPC](https://www.rivet.dev/docs/integrations/trpc)

### Auth
- [Better Auth](https://www.rivet.dev/docs/integrations/better-auth)

### Testing
- [Vitest](https://www.rivet.dev/docs/integrations/vitest)

### AI
- [AI SDK](https://github.com/rivet-dev/rivetkit/tree/9a3d850aee45167eadf249fdbae60129bf37e818/examples/ai-agent) *(On The Roadmap)*

### Sync
- [LiveStore](https://github.com/rivet-dev/rivetkit/issues/908) *(On The Roadmap)*
- [ZeroSync](https://github.com/rivet-dev/rivetkit/issues/909) *(Help Wanted)*
- [TinyBase](https://github.com/rivet-dev/rivetkit/issues/910) *(Help Wanted)*
- [Yjs](https://github.com/rivet-dev/rivetkit/tree/9a3d850aee45167eadf249fdbae60129bf37e818/examples/crdt) *(Help Wanted)*

## Built-In Observability at Scale

Debug and monitor your Rivet applications with powerful built-in tools:

- **Live State Inspection**: View and edit your actor state in real-time
- **Event Monitoring**: Track all events and messages in your system
- **REPL**: Debug your actors interactively
- **Connection Inspection**: Monitor active connections and their state

## Community & Support

Join thousands of developers building with Rivet Actors today:

- [Discord](https://rivet.dev/discord) - Chat with the community
- [X/Twitter](https://x.com/rivet_dev) - Follow for updates
- [Bluesky](https://bsky.app/profile/www.rivet.dev) - Follow for updates
- [GitHub Discussions](https://github.com/www.rivet.dev/rivet/discussions) - Ask questions and share ideas
- [GitHub Issues](https://github.com/www.rivet.dev/rivet/issues) - Report bugs and request features
- [Talk to an engineer](https://www.rivet.dev/talk-to-an-engineer) - Discuss your technical needs, current stack, and how Rivet can help with your infrastructure challenges

## License

[Apache 2.0](LICENSE)

