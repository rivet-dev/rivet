<div align="center">
  <a href="https://www.rivet.dev">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="./.github/media/logo/icon-text-white.svg" alt="Rivet">
      <img src="./.github/media/logo/icon-text-black.svg" alt="Rivet" height="75">
    </picture>
  </a>
  <br/>
  <br/>
  <h3>Fast, high-density, scalable orchestration for agentic workloads.</h3>
  <p>
    Instead of a heavyweight sandbox per agent, each agent gets a lightweight Rivet Actor backed by S3.<br/>
    4.99 ms p99 cold starts and 72 KB per Actor. Open source and self-hostable.
  </p>
  <p>
    <a href="https://www.rivet.dev/docs">Quickstart</a> •
    <a href="https://www.rivet.dev/actors/docs">Documentation</a> •
    <a href="https://www.rivet.dev/changelog">Changelog</a> •
    <a href="https://www.rivet.dev/discord">Discord</a> •
    <a href="https://x.com/rivet_dev">X</a>
  </p>
</div>

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

- [Node.js & Bun](https://www.rivet.dev/actors/docs/quickstart/backend)
- [React](https://www.rivet.dev/actors/docs/quickstart/react)
- [Next.js](https://www.rivet.dev/actors/docs/quickstart/next-js)
- [Rust](https://www.rivet.dev/actors/docs/quickstart/rust)
- [Effect](https://www.rivet.dev/actors/docs/quickstart/effect)

[View documentation →](https://www.rivet.dev/docs)

</td>
</tr>
</table>

## What is Rivet?

Rivet is an orchestrator for agentic workloads built on the actor model. Each Rivet Actor is a long-lived process with in-memory state, a SQLite database, WebSockets for realtime, and scheduling, addressed by key. Create one per agent, session, user, or tenant, and run it on plain Node.js, Bun, or Rust.

- **72 KB per Actor**: Actors share a node process instead of each booting a sandbox, so one server holds hundreds of thousands.
- **4.99 ms p99 cold starts**: No image pull and no VM boot.
- **SQLite backed by S3, at SSD speeds**: 1.07 ms durable writes and 1.36 ms reads at p99, for about 10.4× less than replicated SSD storage.
- **Hibernates when idle**: Idle Actors cost nothing and wake on the next request with nothing lost.

**Backend**

```typescript
import { actor } from "rivetkit";
import { db } from "rivetkit/db";

const agent = actor({
  // Per-Actor SQLite database, persisted across restarts and hibernation
  db: db({
    onMigrate: async (db) => {
      // Apply the SQLite schema
      await db.execute(`CREATE TABLE IF NOT EXISTS messages (role TEXT, content TEXT)`);
    },
  }),

  actions: {
    chat: async (c, text: string) => {
      // Durably save the message
      await c.db.execute("INSERT INTO messages VALUES (?, ?)", "user", text);
      const messages = await c.db.execute("SELECT role, content FROM messages");

      const response = streamText({ model: openai("gpt-5"), messages });

      // Stream tokens to every connected client for multiplayer
      for await (const delta of response.textStream) {
        c.broadcast("token", delta);
      }

      // Durably save the response
      await c.db.execute("INSERT INTO messages VALUES (?, ?)", "assistant", await response.text);
    },
  },
});
```

**Client** (frontend or backend)

```typescript
// Connect to an Actor
const agent = client.agent.getOrCreate("agent-123").connect();

// Listen for realtime events
agent.on("token", delta => process.stdout.write(delta));

// Call an action on the Actor
await agent.chat("how many r's in strawberry?");
```

## Actor types

Every product Rivet ships is an Actor, scheduled by the same control plane.

- **[Actors](https://www.rivet.dev/actors/docs)**: Long-lived processes with state, realtime, and scheduling built in.
- **[Agents](https://www.rivet.dev/agents/docs)**: A durable process per agent, with memory that survives restarts.
- **[Workflows](https://www.rivet.dev/workflows/docs)**: Multi-step operations that replay instead of starting over.
- **[agentOS](https://www.rivet.dev/agentos/docs)**: Run the code your agent writes, securely, with granular control over network access, filesystem access, and resource limits.
- **[Dynamic Apps](https://www.rivet.dev/dynamic-apps/docs)** (preview): A backend per user, deployed the moment it is generated.

## Built for everything agents need

- **Coding agents**: One Actor per agent, with a durable process and state that survives restarts.
- **Agent app builders**: Each generated backend deploys as its own Actor that scales to zero.
- **Agents in your VPC**: Run agents next to your databases and APIs, so no data leaves your cloud.
- **Realtime apps**: One Actor per document, room, or channel, broadcasting changes to every client.

## How Actors compare to sandboxes

| | Rivet Actor | Sandbox |
|---|:---:|:---:|
| **Cold start (p99)** | **4.99 ms** | 1.70 s |
| **Memory per instance** | **72.4 KB** | 512 MiB |
| **Storage** | **SQLite backed by S3**, about 10.4× cheaper than replicated SSDs | Disk per sandbox |

<details>
<summary>Benchmark details & methodology</summary>

- **Rivet numbers** come from the published [Rivet Actors vs Cloudflare Durable Objects benchmark](https://github.com/rivet-dev/bench-rivet-actors-vs-durable-objects), run on Rivet 3.0.3. Its repository has the raw samples and the scripts to reproduce them.
- **Sandbox numbers** are a documented default memory allocation and a third-party cold start benchmark.
- **Storage cost** compares S3 Standard with three Amazon EBS gp3 copies at AWS list prices.
- Sources, dates, and the full method are at [rivet.dev/docs/benchmarking](https://www.rivet.dev/docs/benchmarking/).

</details>

## Features

- **Fault tolerance**: If a node goes down, its Actors move elsewhere with their state intact.
- **HTTP & WebSocket networking**: Call an Actor over HTTP or hold a live WebSocket to it.
- **End-to-end type safety**: Typed clients for your backend and React hooks for your frontend.
- **Cron & scheduling**: Wake an Actor on a schedule or at a time it sets for itself.
- **Actor-to-Actor calls**: Actors call each other by key across the cluster.
- **OpenTelemetry**: Traces, metrics, and logs in OTel format.

## Deployment Options

<table>
<tr>
<td width="33%" valign="top">

### Local & Self-Host

Run Rivet locally while you build. In production, run the control plane on Docker, Kubernetes, or a VM on AWS, GCP, Azure, or Hetzner. S3 is the only dependency.

```bash
npm install rivetkit
```

[Open the quickstart →](https://www.rivet.dev/actors/docs/quickstart/backend)
[Read self-hosting docs →](https://www.rivet.dev/docs/deploy/self-host/control-plane/)

</td>
<td width="33%" valign="top">

### Rivet Cloud

Fully managed, dedicated clusters on AWS or GCP in any region. Run your nodes on Rivet Cloud or bring your own.

[Get started with Rivet Cloud →](https://dashboard.rivet.dev)

</td>
<td width="33%" valign="top">

### BYOC

Run the control plane inside your own VPC on AWS, GCP, or Azure, fully managed by Rivet. Your data never leaves your cloud.

[Read the BYOC docs →](https://www.rivet.dev/docs/deploy/byoc/)

</td>
</tr>
</table>

Every option runs the same control plane, from local dev to production. [Talk to an engineer →](https://www.rivet.dev/talk-to-an-engineer)

## Works with the tools you already use

Typed SDKs for your backend and React hooks for your frontend. Rivet runs on native Node.js and Bun with no custom runtime, so your existing packages, tooling, and tests work unchanged.

**Deploy nodes on**: [Docker Compose](https://www.rivet.dev/docs/deploy/self-host/nodes/docker-compose/) • [Kubernetes](https://www.rivet.dev/docs/deploy/self-host/nodes/kubernetes/) • [VM & bare metal](https://www.rivet.dev/docs/deploy/self-host/nodes/vm/) • [Hetzner](https://www.rivet.dev/docs/deploy/self-host/nodes/hetzner/) • [AWS ECS](https://www.rivet.dev/docs/deploy/self-host/nodes/aws-ecs/) • [Railway](https://www.rivet.dev/docs/deploy/self-host/nodes/railway/) • [Render](https://www.rivet.dev/docs/deploy/self-host/nodes/render/)

**Frameworks**: [React](https://www.rivet.dev/actors/docs/clients/react) • [Effect](./examples/ai-agent-effect)

**Runtimes**: [Node.js](https://www.rivet.dev/actors/docs/quickstart/backend) • [Bun](https://www.rivet.dev/actors/docs/quickstart/backend) • [Rust](https://www.rivet.dev/actors/docs/quickstart/rust)

**Tools**: [Vitest](https://www.rivet.dev/actors/docs/testing) • [OpenTelemetry](https://www.rivet.dev/actors/docs/general/tracing) • [Pino](https://www.rivet.dev/actors/docs/general/logging) • [AI SDK](./examples/ai-agent) • [OpenAPI](./rivetkit-openapi) • [AsyncAPI](./rivetkit-asyncapi)

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
| ↳ [Gasoline](./engine/packages/gasoline) | Durable execution engine |
| ↳ [Guard](./engine/packages/guard) | Traffic routing proxy |
| ↳ [Epoxy](./engine/packages/epoxy) | Multi-region KV store (EPaxos) |
| [Dashboard](./frontend) | Inspector for debugging Actors |
| [Documentation](./docs) | Source for [rivet.dev/actors/docs](https://www.rivet.dev/actors/docs) |
| [Examples](./examples) | Runnable examples |

## Community

- [Discord](https://www.rivet.dev/discord) - Chat with the community
- [X/Twitter](https://x.com/rivet_dev) - Follow for updates
- [Bluesky](https://bsky.app/profile/www.rivet.dev) - Follow for updates
- [GitHub Discussions](https://github.com/rivet-dev/rivet/discussions) - Ask questions
- [GitHub Issues](https://github.com/rivet-dev/rivet/issues) - Report bugs
- [Talk to an engineer](https://www.rivet.dev/talk-to-an-engineer) - Discuss your use case

## License

[Apache 2.0](LICENSE)
