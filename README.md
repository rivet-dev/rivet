<div align="center">
  <a href="https://www.rivet.dev">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="./.github/media/logo/icon-text-white.svg" alt="Rivet">
      <img src="./.github/media/logo/icon-text-black.svg" alt="Rivet" height="75">
    </picture>
  </a>
  <br/>
  <br/>
  <h3>Fast, high-density, scalable orchestration with Rivet Actors.<br/>Built on S3 at SSD speeds.</h3>
  <p>
    4.99 ms p99 cold starts, 72 KB RAM per Actor, scales to billions
  </p>
  <p>
    <a href="https://www.rivet.dev/docs">Quickstart</a> •
    <a href="https://www.rivet.dev/actors/docs">Documentation</a> •
    <a href="https://www.rivet.dev/blog">Blog</a> •
    <a href="https://www.rivet.dev/discord">Discord</a> •
    <a href="https://x.com/rivet_dev">X</a>
  </p>
</div>

## What is Rivet?

Rivet Actors are long-lived processes with in-memory state, a SQLite database, WebSockets, and scheduling. You can create one for each agent, user, session, or tenant.

- **72 KB per Actor**: Actors share a node process instead of each booting a sandbox, so one server holds hundreds of thousands.
- **4.99 ms p99 cold starts**: No image pull and no VM boot.
- **Built for scale**: All data lives in S3, so a cluster scales to billions of Actors.
- **Hibernates when idle, wakes on demand**: Idle Actors cost nothing and wake on the next request with nothing lost.
- **SQLite backed by S3, at SSD speeds**: 1.07 ms durable writes and 1.36 ms reads at p99, for about 10.4× less than replicated SSD storage.

### Use cases

- **[Durable & multiplayer AI agents](https://www.rivet.dev/guides/ai-agent)**: memory that survives restarts, shared live with every client
- **[Workflows](https://www.rivet.dev/workflows/docs)**: multi-step operations that replay instead of starting over
- **[AI-generated apps](https://www.rivet.dev/dynamic-apps/docs)**: run the apps and Actors your agent generates
- **[Collaborative editing](https://www.rivet.dev/guides/collaborative-text-editor)**: one Actor per document, synced to every editor

### Example

A minimal agent written from scratch. For a ready-made agent, use [Pi Durable](https://www.rivet.dev/agents/docs/quickstart/).

```typescript
// server.ts
import { actor, setup } from "rivetkit";
import { db } from "rivetkit/db";
import { streamText } from "ai";
import { openai } from "@ai-sdk/openai";

// One Actor per agent, created on first use: agent.getOrCreate(["agent-123"])
export const agent = actor({
  // 1. SQLite: every agent has its own database, so history survives restarts
  db: db({
    onMigrate: async (db) => {
      await db.execute("CREATE TABLE IF NOT EXISTS messages (role TEXT, content TEXT)");
    },
  }),

  // 2. In-memory state: plain variables that live with the Actor, with no database round trip
  createVars: () => ({ online: new Map<string, string>() }),

  // 3. WebSockets: each client holds a live connection, and the Actor pushes to all of them
  onDisconnect: (c, conn) => {
    c.vars.online.delete(conn.id);
    c.broadcast("online", [...c.vars.online.values()]);
  },

  actions: {
    join: (c, user: string) => {
      c.vars.online.set(c.conn.id, user);
      c.broadcast("online", [...c.vars.online.values()]);
    },

    sendMessage: async (c, text: string) => {
      // Save the message
      await c.db.execute("INSERT INTO messages VALUES (?, ?)", "user", text);
      const messages = await c.db.execute("SELECT role, content FROM messages");

      // Stream tokens to every connected client
      const response = streamText({ model: openai("gpt-5"), messages });
      for await (const delta of response.textStream) {
        c.broadcast("token", delta);
      }

      // Save the response
      await c.db.execute("INSERT INTO messages VALUES (?, ?)", "assistant", await response.text);
    },
  },
});

export const registry = setup({ use: { agent } });
```

```typescript
// client.ts (frontend or backend)
import { createClient } from "rivetkit/client";
import type { registry } from "./server";

const client = createClient<typeof registry>();

// Open a WebSocket connection to the agent
const agent = client.agent.getOrCreate(["agent-123"]).connect();

// Listen for realtime events
agent.on("online", users => console.log("online:", users));
agent.on("token", delta => process.stdout.write(delta));

// Call actions on the Actor
await agent.join("nathan");
await agent.sendMessage("how many r's in strawberry?");
```

### Why run agents on Rivet Actors?

Each Actor gives the agent a **long-lived process** for long agent turns, **in-memory state** for context, a **WebSocket server** for multiplayer, **durable SQLite storage** for session transcripts and agent artifacts, and **hibernation** when idle.

Most of an agent's time is spent **waiting on inference**. When the agent loop runs inside a sandbox, you pay for a whole VM that sits **mostly idle** with low CPU usage.

Rivet Actors run the agent loop outside of the sandbox instead, often called **"separating the head from the hands."** The agent's state, memory, and context live in the Actor, and it reaches the sandbox through tools only when it needs to run code. This uses a fraction of the memory and limits the **blast radius** and **trust boundary** of the code it runs. Read more about [running the harness outside of the sandbox](https://www.rivet.dev/blog/2026-07-27-run-your-harness-outside-the-sandbox/).

### Do I need a sandbox with a Rivet Actor?

Often not. An agent needs a sandbox only to run code it writes. Developers on Rivet use three patterns:

- **Agents with tools**: Many agents only need tool calls, MCP, and APIs, such as customer support or personal agents. No sandbox needed.
- **Agents with a lightweight sandbox**: [agentOS](https://www.rivet.dev/agentos/docs) provides Linux-compatible sandboxing for JS and Python using V8 and WebAssembly, with full POSIX filesystems, networking stacks, and process trees. It covers most workloads, like data analysis, file processing, evals, and code mode.
- **Agents with a full sandbox**: Keep your existing sandbox provider and expose its shell and filesystem to the agent as tools, such as a `bash` tool that runs commands in the sandbox. The Actor reattaches to it across sleeps and restarts. See the [sandbox coding agent example](./examples/sandbox-coding-agent).

## Getting Started

### Use with your coding agent

Paste this prompt into Claude Code, Cursor, Codex, or any coding agent:

```text
Run npx skills add rivet-dev/skills, then ask me what to build and run it locally.
```

### Start from scratch

Rivet ships libraries built on top of Actors for common use cases.

- **[Actors](https://www.rivet.dev/actors/docs)**: Long-lived processes with state, realtime, and scheduling built in.
- **[Agents](https://www.rivet.dev/agents/docs)**: Durable agents built on Actors, with memory that survives restarts.
- **[Workflows](https://www.rivet.dev/workflows/docs)**: Multi-step operations built on Actors that replay instead of starting over.
- **[agentOS](https://www.rivet.dev/agentos/docs)**: Run the code your agent writes, securely, with granular control over network access, filesystem access, and resource limits.
- **[Dynamic Apps](https://www.rivet.dev/dynamic-apps/docs)** (preview): A backend per user, deployed the moment it is generated.

## How Actors compare to sandboxes

| Metric | Rivet Actor | Sandbox |
| --- | --- | --- |
| Cold start (p99) | **4.99 ms** | 1.70 s |
| Memory per instance | **72.4 KB** | 512 MiB |
| Storage | **SQLite on S3** | Disk per sandbox |
| Storage cost | **10.4× cheaper** | Replicated SSDs |

<details>
<summary>Benchmark details & methodology</summary>

- **Rivet numbers** come from the published [Rivet Actors vs Cloudflare Durable Objects benchmark](https://github.com/rivet-dev/bench-rivet-actors-vs-durable-objects), run on Rivet 3.0.3. Its repository has the raw samples and the scripts to reproduce them.
- **Sandbox numbers** are for [E2B](https://e2b.dev): 512 MiB is its default memory allocation from [E2B's billing documentation](https://docs.e2b.dev/billing) as of November 4, 2026, and 1.70 s is the p99 cold start from [ComputeSDK's October 30, 2026 sandbox benchmark](https://www.computesdk.com/benchmarks/sandboxes/burst-tti).
- **Storage cost** compares S3 Standard with three Amazon EBS gp3 copies at AWS list prices.
- Sources, dates, and the full method are at [rivet.dev/docs/benchmarking](https://www.rivet.dev/docs/benchmarking/).

</details>

## Features

- **Fault tolerance**: If a node goes down, its Actors move elsewhere with their state intact.
- **HTTP & WebSocket networking**: Call an Actor over HTTP or hold a live WebSocket to it.
- **End-to-end type safety**: Typed clients for your backend and React hooks for your frontend.
- **Cron & scheduling**: Wake an Actor on a schedule or at a time it sets for itself.
- **Actor-to-Actor calls**: Actors call each other by key across the cluster.
- **Scoped JWTs**: Give each client a [JWT](https://www.rivet.dev/docs/jwt) scoped to the Actors it may reach.
- **OpenTelemetry**: Traces, metrics, and logs in OTel format.

## Deployment Options

| Option | Description | Get started |
| --- | --- | --- |
| **Local & self-host** | Run Rivet locally while you build. In production, run the control plane on Docker, Kubernetes, or a VM. S3 is the only dependency. | [Self-hosting docs](https://www.rivet.dev/docs/deploy/self-host/control-plane/) |
| **Rivet Cloud** | Fully managed, dedicated clusters on AWS or GCP in any region. Run your nodes on Rivet Cloud or bring your own. | [Dashboard](https://dashboard.rivet.dev) |
| **BYOC** | The control plane runs inside your own VPC on AWS, GCP, or Azure, fully managed by Rivet. Your data never leaves your cloud. | [BYOC docs](https://www.rivet.dev/docs/deploy/byoc/) |

Every option runs the same control plane, from local dev to production. [Talk to an engineer →](https://www.rivet.dev/talk-to-an-engineer)

## Works with the tools you already use

Typed SDKs for your backend and React hooks for your frontend. Rivet runs on native Node.js and Bun with no custom runtime, so your existing packages, tooling, and tests work unchanged.

**Deploy nodes on**: [Docker Compose](https://www.rivet.dev/docs/deploy/self-host/nodes/docker-compose/) • [Kubernetes](https://www.rivet.dev/docs/deploy/self-host/nodes/kubernetes/) • [VM & bare metal](https://www.rivet.dev/docs/deploy/self-host/nodes/vm/) • [Hetzner](https://www.rivet.dev/docs/deploy/self-host/nodes/hetzner/) • [AWS ECS](https://www.rivet.dev/docs/deploy/self-host/nodes/aws-ecs/) • [Railway](https://www.rivet.dev/docs/deploy/self-host/nodes/railway/) • [Render](https://www.rivet.dev/docs/deploy/self-host/nodes/render/)

**Frameworks**: [React](https://www.rivet.dev/actors/docs/clients/react) • [Effect](./examples/ai-agent-effect)

**Runtimes**: [Node.js](https://www.rivet.dev/actors/docs/quickstart/backend) • [Bun](https://www.rivet.dev/actors/docs/quickstart/backend) • [Rust](https://www.rivet.dev/actors/docs/quickstart/rust)

**Tools**: [Vitest](https://www.rivet.dev/actors/docs/testing) • [OpenTelemetry](https://www.rivet.dev/docs/tracing) • [Pino](https://www.rivet.dev/actors/docs/logging) • [AI SDK](./examples/ai-agent) • [OpenAPI](./rivetkit-openapi) • [AsyncAPI](./rivetkit-asyncapi)

[Request an integration →](https://github.com/rivet-dev/rivet/issues/new)

## FAQ

<details>
<summary><b>Does Rivet have BYOC?</b></summary>

Yes. Rivet deploys into your own cloud account, including air-gapped environments: the control plane and storage run inside your VPC while Rivet operates them. See the [BYOC docs](https://www.rivet.dev/docs/deploy/byoc/).

</details>

<details>
<summary><b>Does Rivet have a cloud?</b></summary>

Yes. [Rivet Cloud](https://dashboard.rivet.dev) runs fully managed, dedicated clusters on AWS or GCP in any region. It can run your app's Docker image as [nodes on Rivet Cloud](https://www.rivet.dev/docs/deploy/cloud/compute/), or you can [bring your own nodes](https://www.rivet.dev/docs/deploy/self-host/nodes/).

</details>

<details>
<summary><b>Can I self-host Rivet?</b></summary>

Yes. Rivet is fully self-hostable; see the [self-hosting docs](https://www.rivet.dev/docs/deploy/self-host/control-plane/). The control plane runs anywhere a container runs, with guides for Docker Compose, Kubernetes, VMs and bare metal, Hetzner, Railway, Render, and AWS ECS. For enterprise support with a self-hosted deployment, [contact us](https://www.rivet.dev/enterprise/).

</details>

<details>
<summary><b>Do I still need Postgres, FoundationDB, or NATS?</b></summary>

No. An object storage bucket is the only dependency: there is no database to run and no message bus, and control plane instances find each other through the bucket. Rivet works with Amazon S3, Google Cloud Storage, Azure Blob Storage, and any S3-compatible store such as MinIO, Cloudflare R2, or Hetzner Object Storage. A local filesystem backend is available for trying Rivet on one machine.

</details>

<details>
<summary><b>Is Rivet open-source?</b></summary>

Yes. Rivet is open source under the permissive Apache 2.0 license. You're looking at it.

</details>

<details>
<summary><b>Are Rivet Actors only for agents?</b></summary>

No. Rivet is a general actor platform at its core, with nothing specific to agents. For agents, you can use the optional [agent library](https://www.rivet.dev/agents/docs/quickstart/), which runs Pi Durable on Rivet Actors, or write your own agent from scratch.

</details>

<details>
<summary><b>How does Rivet compare to sandboxes?</b></summary>

A sandbox boots a VM or container per agent, which is slow to start and sits mostly idle. A Rivet Actor runs inside a shared node process: 72 KB and 4.99 ms p99 to start, versus 512 MiB and 1.70 s for a sandbox, and it hibernates when idle. To run untrusted code your agent writes, use [agentOS](https://www.rivet.dev/agentos/docs).

</details>

<details>
<summary><b>How does Rivet compare to Cloudflare Durable Objects?</b></summary>

Rivet is often used as an open-source alternative to Cloudflare Durable Objects that is easy to self-host and scales. It also fixes many of the constraints of Durable Objects: no 128 MB memory cap, bottomless storage instead of a 10 GB SQLite limit, Actors are not randomly evicted, and you control the runtime because Actors run on vanilla Node.js or Bun instead of a custom runtime. Rivet also adds built-in connection handling, event broadcasting, durable scheduling and cron, full Vitest support, lifecycle hooks for draining, and control over Actor upgrades. See the [Cloudflare Durable Objects comparison](https://www.rivet.dev/actors/compare/rivet-actors-vs-cloudflare-durable-objects/).

</details>

<details>
<summary><b>Does Rivet use a custom V8 runtime?</b></summary>

No. Rivet runs on native Node.js and Bun, so your existing packages, tooling, and tests work unchanged.

</details>

<details>
<summary><b>Is Rivet like Erlang, Akka, or Orleans?</b></summary>

Rivet implements the virtual actor pattern, which is closest to Orleans, but focuses on what modern workloads need: SQLite persistence instead of simple JSON stores and support for modern runtimes. Erlang and Akka do not provide virtual actors out of the box, though ecosystem packages exist that offer functionality comparable to Rivet Actors.

</details>

<details>
<summary><b>Does Rivet require microVMs, gVisor, or nested virtualization?</b></summary>

No. Rivet only needs a standard process that connects to the control plane over WebSocket. That process can be Node.js, Bun, Rust, or anything else.

</details>

## Projects in This Repository

| Project | Description |
|---------|-------------|
| [RivetKit TypeScript](./rivetkit-typescript) | Client & server library for building Actors |
| [RivetKit Rust](./rivetkit-rust) | Rust SDK |
| [RivetKit Python](./rivetkit-python) | Python client (experimental) |
| [RivetKit Swift](./rivetkit-swift) | Swift client (experimental) |
| [Control Plane](./engine) | Rust control plane that schedules, routes, and persists Actors |
| ↳ [Pegboard](./engine/packages/pegboard) | Actor scheduling & networking |
| ↳ [Guard](./engine/packages/guard) | Traffic routing proxy |
| ↳ [Epoxy](./engine/packages/epoxy) | Multi-region KV store (EPaxos) |
| [Documentation](./docs) | Source for [rivet.dev/actors/docs](https://www.rivet.dev/actors/docs) |
| [Examples](./examples) | Runnable examples |

## Community

- [Discord](https://www.rivet.dev/discord) - Chat with the community
- [X/Twitter](https://x.com/rivet_dev) - Follow for updates
- [GitHub Discussions](https://github.com/rivet-dev/rivet/discussions) - Ask questions
- [GitHub Issues](https://github.com/rivet-dev/rivet/issues) - Report bugs
- [Talk to an engineer](https://www.rivet.dev/talk-to-an-engineer) - Discuss your use case

## License

[Apache 2.0](LICENSE)
