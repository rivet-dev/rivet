<div align="center">
  <a href="https://www.rivet.dev">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="./.github/media/logo/icon-text-white.svg" alt="Rivet">
      <img src="./.github/media/logo/icon-text-black.svg" alt="Rivet" height="75">
    </picture>
  </a>
  <br/>
  <br/>
  <h3>The primitive for software that thinks.</h3>
  <p>
    <a href="https://www.rivet.dev/docs">Quickstart</a> •
    <a href="https://www.rivet.dev/docs/actors">Documentation</a> •
    <a href="https://www.rivet.dev/changelog">Changelog</a> •
    <a href="https://www.rivet.dev/discord">Discord</a> •
    <a href="https://x.com/rivet_dev">X</a>
  </p>
</div>

## What is Rivet?

Rivet Actors are a serverless primitive for stateful workloads. Each actor has built-in state, storage, workflows, scheduling, and WebSockets — everything needed to build the next generation of software.

```typescript
import { actor } from "rivetkit";

export const chatRoom = actor({
  // In-memory state, persisted automatically
  state: { messages: [] },

  // Type-safe RPC
  actions: {
    sendMessage: (c, user, text) => {
      c.state.messages.push({ user, text });
      c.broadcast("newMessage", { user, text });
    },
  },
});
```

One Actor per agent, per session, per user — each with everything it needs built in.

## Built-In Features

Every Rivet Actor comes with:

| Feature | Description |
|---------|-------------|
| **In-memory state** | Co-located with compute for instant reads and writes |
| **SQLite or JSON persistence** | Storage that survives restarts and deploys |
| **Runs indefinitely, sleeps when idle** | Long-lived when active, hibernates when idle |
| **Scales infinitely, scales to zero** | Supports bursty workloads, cost-efficient |
| **WebSockets** | Real-time bidirectional streaming built in |
| **Workflows** | Multi-step operations with automatic retries |
| **Queues** | Durable message queues for reliable async processing |
| **Scheduling** | Timers and cron jobs within your actor |

## Use Cases

Rivet is one primitive that adapts to agents, workflows, collaboration, and more.

| Use Case | Description |
|----------|-------------|
| **AI Agent** | Each agent runs as its own actor with persistent context and memory |
| **Sandbox Orchestration** | Coordinate sandbox sessions, queue work, and schedule cleanup |
| **Workflows** | Multi-step operations with automatic retries and durable state |
| **Collaborative Documents** | Real-time editing where each document is an actor |
| **Per-Tenant Database** | One actor per tenant with low-latency in-memory reads |
| **Chat** | One actor per room with in-memory state and realtime delivery |

## Start Local. Scale to Millions.

Three options, same API. Pick what works for you.

<table>
<tr>
<td width="33%" valign="top">

### Self-Host

Single Rust binary or Docker container. Works with Postgres, file system, or FoundationDB.

```bash
docker run -p 6420:6420 rivetkit/engine
```

[Self-hosting documentation →](https://www.rivet.dev/docs/self-hosting/)

</td>
<td width="33%" valign="top">

### Rivet Cloud

Fully managed. Global edge network. Connects to your existing cloud — Vercel, Railway, AWS, wherever you already deploy.

[Sign up →](https://hub.rivet.dev)

</td>
<td width="33%" valign="top">

### Open Source

Apache 2.0. Audit the code, contribute features, run it however you want.

[View on GitHub →](https://github.com/rivet-dev/rivet)

</td>
</tr>
</table>

## Getting Started

<table>
<tr>
<td width="50%" valign="top">

### Use with Your Coding Agent

Give your coding agent the Rivet skills to create examples or integrate into existing projects:

```bash
npx skills add rivet-dev/skills
```

Works with Claude Code, Cursor, Windsurf, and other AI coding tools.

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

## Integrations

**Frameworks**: [Hono](https://github.com/rivet-dev/rivet/tree/main/examples/hono) • [Elysia](https://github.com/rivet-dev/rivet/tree/main/examples/elysia) • [tRPC](https://github.com/rivet-dev/rivet/tree/main/examples/trpc)

**Clients**: [JavaScript](https://www.rivet.dev/docs/clients/javascript) • [React](https://www.rivet.dev/docs/clients/react) • [Next.js](https://www.rivet.dev/docs/clients/next-js)

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
