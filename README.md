<div align="center">
  <a href="https://www.rivet.dev">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="./.github/media/logo/icon-text-white.svg" alt="Rivet">
      <img src="./.github/media/logo/icon-text-black.svg" alt="Rivet" height="75">
    </picture>
  </a>
  <br/>
  <br/>
  <h3>Stateful Backends. Finally Solved.</h3>
  <p>
    <a href="https://www.rivet.dev/docs/actors/quickstart">Quickstart</a> •
    <a href="https://www.rivet.dev/docs/actors">Documentation</a> •
    <a href="https://www.rivet.dev/changelog">Changelog</a> •
    <a href="https://www.rivet.dev/discord">Discord</a> •
    <a href="https://x.com/rivet_dev">X</a>
  </p>
</div>

## What is Rivet?

Rivet is an open-source platform for building stateful backends using **Actors**, long-lived processes that keep state in memory alongside compute. No more database round-trips for every request.

```typescript
import { actor, setup } from "rivetkit";

const chatRoom = actor({
  // State persists across requests, restarts, and deployments
  state: { messages: [] as { user: string; text: string }[] },

  actions: {
    send: (c, user: string, text: string) => {
      c.state.messages.push({ user, text });
      c.broadcast("newMessage", { user, text }); // Realtime to all clients
    },
    history: (c) => c.state.messages,
  },
});

export const registry = setup({ use: { chatRoom } });
```

Each actor is like a tiny server with its own memory. Create millions of them (one per user, per document, per game session) and they scale automatically.

## How It Works

<table>
<tr>
<td width="50%" valign="top">

### RivetKit

The TypeScript library for building actors. Works with any backend framework.

```bash
npm install rivetkit
```

Run standalone for development, deploy to Cloudflare Workers, or connect to Rivet Engine for scale.

</td>
<td width="50%" valign="top">

### Rivet Engine

High-performance Rust orchestration layer for production scale. Handles actor lifecycle, state persistence, and multi-region distribution.

- Self-host with Docker or binaries
- Or use [Rivet Cloud](https://rivet.dev/cloud) (managed)

</td>
</tr>
</table>

**You don't need Rivet Engine to get started.** RivetKit runs standalone for development and simpler deployments. Add Rivet Engine when you need to scale.

## Features

- **Stateful Compute**: State lives with compute for sub-millisecond reads/writes, no database queries
- **Realtime Built-In**: WebSocket events without extra infrastructure
- **Hibernation**: Actors sleep when idle and wake instantly with no cold starts
- **Infinitely Scalable**: Auto-scale from zero to millions of actors
- **Fault Tolerant**: Automatic failover with state integrity
- **Type-Safe**: End-to-end TypeScript types from actor to client
- **Run Anywhere**: Your infrastructure, your cloud, your rules

## Getting Started

<table>
<tr>
<td width="50%" valign="top">

### One-Click Templates

- [Chat Room](https://rivet.dev/templates/chat-room) - Realtime messaging
- [AI Agent](https://rivet.dev/templates/ai-agent) - Stateful AI conversations
- [Cursors](https://rivet.dev/templates/cursors) - Collaborative cursors
- [Scheduling](https://rivet.dev/templates/scheduling) - Async processing

[View all templates →](https://rivet.dev/templates)

</td>
<td width="50%" valign="top">

### Start From Scratch

- [Node.js & Bun](https://www.rivet.dev/docs/actors/quickstart/backend)
- [React](https://www.rivet.dev/docs/actors/quickstart/react)
- [Next.js](https://www.rivet.dev/docs/actors/quickstart/next-js)
- [Cloudflare Workers](https://www.rivet.dev/docs/actors/quickstart/cloudflare-workers)

[View documentation →](https://www.rivet.dev/docs)

</td>
</tr>
</table>

## Deploy Anywhere

### Rivet Cloud

Deploy your backend anywhere, then connect to [Rivet Cloud](https://dashboard.rivet.dev) for actor orchestration: [Vercel](https://www.rivet.dev/docs/connect/vercel) • [Railway](https://railway.com/deploy/rivet) • [Kubernetes](https://www.rivet.dev/docs/connect/kubernetes) • [AWS ECS](https://www.rivet.dev/docs/connect/aws-ecs) • [Google Cloud Run](https://www.rivet.dev/docs/connect/gcp-cloud-run) • [Hetzner](https://www.rivet.dev/docs/connect/hetzner) • [VM & Bare Metal](https://www.rivet.dev/docs/connect/vm-and-bare-metal)

### Self-Host Rivet Engine

```bash
# Docker
docker run -p 6420:6420 rivetkit/engine

# macOS (Apple Silicon)
curl -o rivet-engine "https://releases.rivet.dev/rivet/latest/engine/rivet-engine-aarch64-apple-darwin" && chmod +x rivet-engine && ./rivet-engine start

# macOS (Intel)
curl -o rivet-engine "https://releases.rivet.dev/rivet/latest/engine/rivet-engine-x86_64-apple-darwin" && chmod +x rivet-engine && ./rivet-engine start

# Linux (x86)
curl -o rivet-engine "https://releases.rivet.dev/rivet/latest/engine/rivet-engine-x86_64-unknown-linux-musl" && chmod +x rivet-engine && ./rivet-engine start
```

[Self-hosting documentation →](https://www.rivet.dev/docs/self-hosting/)

## Integrations

**Frameworks**: [Hono](https://rivet.dev/templates/hono) • [Elysia](https://rivet.dev/templates/elysia) • [tRPC](https://rivet.dev/templates/trpc)

**Clients**: [JavaScript](https://www.rivet.dev/docs/clients/javascript) • [React](https://www.rivet.dev/docs/clients/react) • [Next.js](https://www.rivet.dev/docs/clients/next-js) • [Rust](https://www.rivet.dev/docs/clients/rust)

[Request an integration →](https://github.com/rivet-dev/rivet/issues/new)

## Projects in This Repository

| Project | Description |
|---------|-------------|
| [RivetKit TypeScript](./rivetkit-typescript) | Client & server library for building actors |
| [RivetKit Rust](./rivetkit-rust) | Rust client (experimental) |
| [RivetKit Python](./rivetkit-python) | Python client (experimental) |
| [Rivet Engine](./engine) | Rust orchestration engine |
| ↳ [Pegboard](./engine/packages/pegboard) | Actor orchestrator & networking |
| ↳ [Gasoline](./engine/packages/gasoline) | Durable execution engine |
| ↳ [Guard](./engine/packages/guard) | Traffic routing proxy |
| ↳ [Epoxy](./engine/packages/epoxy) | Multi-region KV store (EPaxos) |
| [Dashboard](./frontend) | Inspector for debugging actors |
| [Website](./website) | Source for [rivet.dev](https://www.rivet.dev) |
| [Documentation](./website/src/content/docs) | Source for [rivet.dev/docs](https://www.rivet.dev/docs) |

## Community

- [Discord](https://www.rivet.dev/discord) - Chat with the community
- [X/Twitter](https://x.com/rivet_dev) - Follow for updates
- [Bluesky](https://bsky.app/profile/www.rivet.dev) - Follow for updates
- [GitHub Discussions](https://github.com/rivet-dev/rivet/discussions) - Ask questions
- [GitHub Issues](https://github.com/rivet-dev/rivet/issues) - Report bugs
- [Talk to an engineer](https://www.rivet.dev/talk-to-an-engineer) - Discuss your use case

## License

[Apache 2.0](LICENSE)
