<div align="center">
  <a href="https://www.rivet.dev">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="./.github/media/logo/icon-text-white.svg" alt="Rivet">
      <img src="./.github/media/logo/icon-text-black.svg" alt="Rivet" height="75">
    </picture>
  </a>
  <br/>
  <br/>
  <h3>The primitive for stateful workloads.</h3>
  <p>Built for AI agents, collaborative apps, and durable execution.</p>
  <p>
    <a href="https://www.rivet.dev/docs">Quickstart</a> •
    <a href="https://www.rivet.dev/docs/actors">Documentation</a> •
    <a href="https://www.rivet.dev/changelog">Changelog</a> •
    <a href="https://www.rivet.dev/discord">Discord</a> •
    <a href="https://x.com/rivet_dev">X</a>
  </p>
</div>

## What is Rivet?

Rivet Actors are long-running, lightweight processes designed for stateful workloads. State lives in-memory with automatic persistence. Create one per agent, per session, or per user — with built-in workflows, queues, and scheduling.

**Backend**

```typescript
const agent = actor({
  // In-memory, persisted state for the actor
  state: { messages: [] as Message[] },

  // Long-running actor process
  run: async (c) => {
    // Process incoming messages from the queue
    for await (const msg of c.queue.iter()) {
      c.state.messages.push({ role: "user", content: msg.body.text });
      const response = streamText({ model: openai("gpt-5"), messages: c.state.messages });

      // Stream realtime events to all connected clients
      for await (const delta of response.textStream) {
        c.broadcast("token", delta);
      }

      c.state.messages.push({ role: "assistant", content: await response.text });
    }
  },
});
```

**Client** (frontend or backend)

```typescript
// Connect to an actor
const agent = client.agent.getOrCreate("agent-123").connect();

// Listen for realtime events
agent.on("token", delta => process.stdout.write(delta));

// Send message to actor
await agent.queue.send("how many r's in strawberry?");
```

## Features

One Actor per agent, per session, per user — state, storage, and networking included.

**Rivet provides:**
- **In-memory state** — Co-located with compute for instant reads and writes. Persist with SQLite or BYO database.
- **Runs indefinitely, sleeps when idle** — Long-lived when active, hibernates when idle.
- **Scales infinitely, scales to zero** — Supports bursty workloads and is cost-efficient.
- **Global edge network** — Deploy close to your users or in specific legal jurisdictions without complexity.

**Actors support:**
- **WebSockets** — Real-time bidirectional streaming built in.
- **Workflows** — Multi-step operations with automatic retries.
- **Queues** — Durable message queues for reliable async processing.
- **Scheduling** — Timers and cron jobs within your actor.

## Use Cases

One primitive that adapts to agents, workflows, collaboration, and more.

- **AI Agent** — Each agent runs as its own actor with persistent context, memory, and the ability to schedule tool calls.
- **Sandbox Orchestration** — Coordinate sandbox sessions, queue work, and schedule cleanup in one long-lived actor per workspace.
- **Workflows** — Multi-step operations with automatic retries, scheduling, and durable state across steps.
- **Collaborative Documents** — Real-time collaborative editing where each document is an actor broadcasting changes to all connected users.
- **Per-Tenant Database** — One actor per tenant with low-latency in-memory reads and durable tenant data persistence.
- **Chat** — One actor per room or conversation with in-memory state, persistent history, and realtime delivery.

## How Actors Compare

**Rivet Actors vs. Traditional Infrastructure**

| Metric | Rivet Actor | Kubernetes Pod | Virtual Machine |
|--------|:-----------:|:--------------:|:---------------:|
| **Cold start** | **~20ms** | ~6s | ~30s |
| **Memory per instance** | **~0.6KB** | ~50MB | ~512MB |
| **Idle cost** | **$0** | ~$85/mo (cluster) | ~$5/mo |
| **Horizontal scale** | **Infinite** | ~5k nodes | Manual |
| **Multi-region** | **Global edge** | 1 region | 1 region |

**State**

| Metric | Rivet Actor | Redis | Postgres |
|--------|:-----------:|:-----:|:--------:|
| **Read latency** | **0ms** | ~1ms | ~5ms |

<details>
<summary>Benchmark details & methodology</summary>

**Cold Start**
- **Rivet Actor (~20ms):** Includes durable state init, not just a process spawn. No actor key, so no cross-region locking. Measured with Node.js and FoundationDB.
- **Kubernetes Pod (~6s):** Node.js 24 Alpine image (56MB compressed) on AWS EKS with a pre-provisioned m5.large node. Breakdown: ~1s image pull and extraction, ~3-4s scheduling and container runtime setup, ~1s container start.
- **Virtual Machine (~30s):** AWS EC2 t3.nano from launch to SSH-ready, using an Amazon Linux 2 AMI. t3.nano is the smallest available EC2 instance (512MB RAM).

**Memory Per Instance**
- **Rivet Actor (~0.6KB):** RSS delta divided by actor count, measured by spawning 10,000 actors in Node.js v24 on Linux x86.
- **Kubernetes Pod (~50MB):** Minimum idle Node.js container on Linux x86: Node.js v24 runtime (~43MB RSS), containerd-shim (~3MB), pause container (~1MB), and kubelet per-pod tracking (~2MB).
- **Virtual Machine (~512MB):** AWS EC2 t3.nano, the smallest available EC2 instance with 512MB allocated memory.

**Read Latency**
- **Rivet Actor (0ms):** State is read from co-located SQLite/KV storage on the same machine as the actor, with no network round-trip.
- **Redis (~1ms):** AWS ElastiCache Redis (cache.t3.micro) in the same availability zone as the application.
- **Postgres (~5ms):** AWS RDS PostgreSQL (db.t3.micro) in the same availability zone as the application.

**Idle Cost**
- **Rivet Actor ($0):** Assumes Rivet Actors running on a serverless platform. Actors scale to zero with no idle infrastructure costs. Traditional container deployments may incur idle costs.
- **Virtual Machine (~$5/mo):** AWS EC2 t3.nano ($0.0052/hr compute + $1.60/mo for 20GB gp3 storage) running 24/7. t3.nano is the smallest available EC2 instance (512MB RAM).
- **Kubernetes Cluster (~$85/mo):** AWS EKS control plane ($73/mo) plus a single t3.nano worker node with 20GB gp3 storage, running 24/7.

**Horizontal Scale**
- **Rivet Actors (Infinite):** Scale linearly by adding nodes with no single cluster size limit.
- **Kubernetes (~5k nodes):** Officially supports clusters of up to 5,000 nodes per the Kubernetes scalability documentation.

**Multi-Region**
- **Rivet (Global edge network):** Automatically spawns actors near your users and handles routing across regions.

</details>

## Built-In Observability

Powerful debugging and monitoring tools from local development to production at scale.

<p align="center">
  <img src="./.github/media/screenshots/inspector.png" alt="Rivet Inspector" width="800" />
</p>

- **SQLite Viewer** — Browse and query your actor's SQLite database in real-time
- **Workflow State** — Inspect workflow progress, steps, and retries as they execute
- **Event Monitoring** — Track every state change and action as it happens
- **REPL** — Call actions, subscribe to events, and interact directly with your code


## Deployment Options

RivetKit is a library. Connect it to Rivet Cloud or self-host when you need scaling, fault tolerance, and observability.

<table>
<tr>
<td width="33%" valign="top">

### Just a Library

Install a package and run locally. No servers, no infrastructure. Actors run in your process during development.

```bash
npm install rivetkit
```

[Get started →](https://www.rivet.dev/docs/actors/quickstart)

</td>
<td width="33%" valign="top">

### Self-Host

Single Rust binary or Docker container. Works with Postgres, file system, or FoundationDB.

```bash
docker run -p 6420:6420 rivetdev/engine
```

[Self-hosting documentation →](https://www.rivet.dev/docs/self-hosting/)

</td>
<td width="33%" valign="top">

### Rivet Cloud

Fully managed. Global edge network. Connects to your existing cloud — Vercel, Railway, AWS, wherever you already deploy.

[Sign up →](https://hub.rivet.dev)

</td>
</tr>
</table>

**Open source, permissively licensed** — Self-hosting matters for enterprise deployments, cloud portability, and avoiding vendor lock-in. Apache 2.0 means you own your infrastructure. [View on GitHub →](https://github.com/rivet-dev/rivet)

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

Serverless, containers, or your own servers — Rivet Actors work with your existing infrastructure, frameworks, and tools.

**Infrastructure**: [Vercel](https://www.rivet.dev/docs/deploy/vercel) • [Railway](https://www.rivet.dev/docs/deploy/railway) • [AWS](https://www.rivet.dev/docs/deploy/aws) • [Docker](https://www.rivet.dev/docs/self-hosting)

**Frameworks**: [React](https://www.rivet.dev/docs/clients/react) • [Next.js](https://www.rivet.dev/docs/clients/next-js) • [Hono](https://github.com/rivet-dev/rivet/tree/main/examples/hono) • [Express](https://github.com/rivet-dev/rivet/tree/main/examples/express) • [Elysia](https://github.com/rivet-dev/rivet/tree/main/examples/elysia) • [tRPC](https://github.com/rivet-dev/rivet/tree/main/examples/trpc)

**Runtimes**: [Node.js](https://www.rivet.dev/docs/actors/quickstart/backend) • [Bun](https://www.rivet.dev/docs/actors/quickstart/backend) • [Deno](https://github.com/rivet-dev/rivet/tree/main/examples/deno) • [Cloudflare Workers](https://www.rivet.dev/docs/actors/quickstart/cloudflare-workers)

**Tools**: [Vitest](https://www.rivet.dev/docs/actors/testing) • [Pino](https://www.rivet.dev/docs/general/logging) • [AI SDK](https://github.com/rivet-dev/rivet/tree/main/examples/ai-agent) • [OpenAPI](https://github.com/rivet-dev/rivet/tree/main/rivetkit-openapi) • [AsyncAPI](https://github.com/rivet-dev/rivet/tree/main/rivetkit-asyncapi)

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
