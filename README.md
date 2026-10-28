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
    Rivet Actors are durable processes for agents, workflows, and sandboxes.<br/>
    6.9 ms cold starts, 72 KB per Actor, billions on one control plane.<br/>
    One Rust binary and a bucket. No database, no message bus. Open source and self-hostable.
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
- [Cloudflare Workers](https://www.rivet.dev/docs/deploy/self-host/workers/cloudflare/)

[View documentation →](https://www.rivet.dev/docs)

</td>
</tr>
</table>

## What is Rivet?

Rivet is an orchestrator for agentic workloads. Where Kubernetes schedules pods, Rivet schedules Actors: long-lived processes with durable state, a SQLite database, a queue, realtime connections, and scheduling, each addressed by key.

Create one Actor per agent, per session, per user, or per tenant. Run it on plain Node.js, Bun, or Rust. Open source and self-hostable.

- **72 KB per Actor**: An Actor lives inside a node process you already run, not in its own container or VM. A server that held hundreds of pods holds hundreds of thousands of Actors.
- **6.9 ms cold starts**: A cold start includes scheduling the Actor, creating its storage, and serving the first request. No image pull, no container boot.
- **Billions of Actors on one control plane**: Actors are scheduled independently, so adding capacity means adding machines. The same control plane runs a thousand Actors or a billion.
- **Hibernates when idle, wakes on demand**: An idle Actor writes its state and unloads. The next request brings it back in 4.4 ms with nothing lost.
- **Keep state with each Actor**: Every Actor gets its own SQLite database and key-value storage. The control plane replicates writes to the disks of several instances before acknowledging them, then moves data to object storage in the background. Storage never runs out and the bucket never sits on the write path.
- **One Rust binary and a bucket**: There is no database to run and no message bus. Control plane instances are identical, run every service, and find each other through the bucket.
- **Run code agents generate at runtime**: Each piece of new, untrusted code gets an Actor of its own, with its own database and filesystem and no reach into anyone else's.
- **Works with the tools you already use**: Typed SDKs for your backend and React hooks for your frontend. No custom runtime, so your existing packages, tooling, and tests work unchanged.

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

## What's new in Rivet 3.0

Rivet 3.0 is built on **WarpDrive**, a new storage engine that gives every Actor bottomless storage on S3 with the speed of an SSD. [Read the announcement →](https://www.rivet.dev/changelog/2026-10-26-introducing-rivet-3-0-preview/)

- **Bottomless storage with the speed of an SSD**: Each Actor's data lives in its own durable volume holding its SQLite database and key-value data. A write is acknowledged once a majority of the instances holding that volume have it on disk, so losing one instance loses no acknowledged write. The bucket receives the durable copy in the background, shortly after, so storage never runs out and the bucket never sits on the write path.
- **No database, no message bus**: Rivet 2.x needed Postgres or FoundationDB plus NATS. In 3.0, WarpDrive replaces the database and the bucket replaces the message broker. Control plane instances find each other through the bucket.
- **Any object store**: Amazon S3, Google Cloud Storage, Azure Blob Storage, or any S3-compatible store such as MinIO, Cloudflare R2, and Hetzner Object Storage. A local filesystem backend is available for trying Rivet on one machine.
- **No code changes**: 3.0 uses the same RivetKit SDK and protocol as 2.x. The upgrade is entirely in the storage engine underneath.

| SQLite on WarpDrive | p50 | p99 |
|---|:---:|:---:|
| **Durable insert** (128-byte row, written to a majority of disks across 3 zones) | **0.69 ms** | 0.80 ms |
| **Warm read** (primary-key read from SQLite's memory cache) | **0.059 ms** | 0.080 ms |

Rivet 3.0 is in preview. Multi-region, serverless, and ACLs have not landed in 3.0 yet; if you depend on them, stay on 2.x, which remains supported. A migration tool will move 2.x deployments to 3.0 with zero downtime.

## Actor types

Every product Rivet ships is an Actor, scheduled by the same control plane.

- **[Actors](https://www.rivet.dev/actors/docs)**: Give every agent a durable process to live in.
- **[Workflows](https://www.rivet.dev/workflows/docs)**: Multi-step operations that replay instead of starting over.
- **[Sandboxes (agentOS)](https://www.rivet.dev/agentos/docs)**: A filesystem, shell, and network for code you did not write.
- **[Dynamic Apps](https://www.rivet.dev/dynamic-apps/docs)**: A backend per user, deployed the moment it is generated.

## Built for everything agents need

- **Coding agents**: One Actor per agent with a durable process, a sandbox, and state that survives restarts.
- **Agent app builders**: Your product generates a whole backend for a user; each one deploys as its own Actor that scales to zero.
- **Company-specific agents**: Run agents inside your VPC next to Postgres, your APIs, and your helpdesk, so no data leaves your cloud.
- **Personal agents**: A long-lived Actor per user with memory, scheduling, and reminders, online for months.
- **Realtime apps**: One Actor per document, room, or channel broadcasting changes to every connected client.

## How Actors compare to Kubernetes

| | Rivet Actor | Kubernetes pod |
|---|:---:|:---:|
| **Cold start** | **6.92 ms** | ~6 s |
| **Wake from hibernation** | **4.37 ms** | ~6 s (new pod) |
| **Memory per instance** | **72.4 KB** | ~200 MB |
| **Memory while idle** | **0 MB** (hibernates) | Always resident |
| **Scale** | **Billions** of Actors on one control plane | ~5k nodes per cluster |
| **Storage** | **Bottomless** SQLite, replicated and moved to your bucket | Persistent volume per pod |

<details>
<summary>Benchmark details & methodology</summary>

- **Cold start (6.92 ms):** Client send to full first reply from a newly created keyed Actor, including scheduling, storage creation, and framework startup: 6.92 ms p50, 8.11 ms p99 over 1,500 samples.
- **Wake (4.37 ms):** Client send to full reply from an Actor asleep for at least 10 seconds: 4.37 ms p50, 6.80 ms p99 over 1,200 samples.
- **Setup:** Measured October 25, 2026 on Azure `eastus2` across three availability zones. Three control plane instances (`Standard_L8as_v5`, local NVMe), one per zone, with every write acknowledged by 2 of 3 instances. One node VM running Node.js 24 in the first zone. Full method and reproduction steps at [rivet.dev/docs/benchmarking](https://www.rivet.dev/docs/benchmarking/).
- **Memory per instance (72.4 KB):** RSS increase per running Actor. Measured with the Rust rivetkit SDK.
- **Kubernetes figures** are typical published values, not a matched benchmark. ~6 s cold start assumes a pre-provisioned node; ~200 MB is a typical idle Node.js pod; ~5k nodes is the documented cluster limit.

</details>

## Features

Routing, scheduling, types, and telemetry, built in.

- **Fault tolerance by design**: A node going down reschedules its Actors elsewhere with their state intact. No replay harness or checkpointing of your own.
- **Keep state with each Actor**: Every Actor gets its own SQLite database and key-value storage. The control plane replicates writes and moves data to object storage, so you never size a disk for an Actor.
- **Multi-region**: Place Actors near the users and data they serve, and route requests to wherever each one currently lives.
- **HTTP & WebSocket networking**: Address an Actor directly over HTTP or hold a live WebSocket to it. No queue or broker in between.
- **End-to-end type safety**: Actor definitions generate their own client types, so a signature change breaks the build rather than production.
- **React SDK**: First-party hooks that subscribe a component to an Actor's state and keep it live as the Actor updates.
- **OpenTelemetry & observability**: Traces, metrics, and structured logs emitted in OTel format, into the collector you already run.
- **Single Rust binary**: The control plane ships as one static binary. Run a few identical instances behind a load balancer with an object storage bucket: no database and no message bus to stand up first.
- **Cron & scheduling**: Wake an Actor on a schedule or at a timestamp it sets for itself, without a separate scheduler.
- **Sleeps when idle**: Idle Actors release their resources and wake with durable state intact when the next request arrives.
- **Actor-to-Actor calls**: Actors address each other by key and call across the cluster as if the other one were local.
- **No Kubernetes operator**: One control plane behind a load balancer, speaking plain HTTP inside your VPC. No CRDs, no operator, no service mesh to keep alive.
- **Open source**: Apache 2.0, self-hostable in full. The managed service runs the same control plane you can run yourself.

## Deployment Options

<table>
<tr>
<td width="33%" valign="top">

### Local & Self-Host

Install Actors and run it locally while you build. When you ship, run the same open-source control plane as a container on vanilla Docker, Kubernetes, or a VM, with a bucket for durable storage.

```bash
npm install rivetkit
```

[Open the quickstart →](https://www.rivet.dev/actors/docs/quickstart/backend)
[Read self-hosting docs →](https://www.rivet.dev/docs/deploy/self-host/control-plane/)

</td>
<td width="33%" valign="top">

### Rivet Cloud

Create a project and deploy. There is nothing to operate. Or bring your own nodes and run Actors on your compute while Rivet Cloud provides the control plane and routing layer.

[Get started with Rivet Cloud →](https://dashboard.rivet.dev)

</td>
<td width="33%" valign="top">

### BYOC

The whole stack runs inside your VPC, and Rivet manages it for you over an outbound connection. Every Actor's data stays in your own account and your own bucket.

[Read the BYOC docs →](https://www.rivet.dev/docs/deploy/byoc/)

</td>
</tr>
</table>

**Every option runs the same binary. The only difference is who operates it.** Self-hosting means running four things: identical control plane instances (one to try it, three or more in production), a persistent disk per instance that holds recent writes and the instance's identity, one bucket per cluster on S3, Google Cloud Storage, Azure Blob Storage, or any S3-compatible store, and the nodes that run your Actors. There is no database to run and no message bus. Local dev matches production: the same binary, APIs, and control-plane behavior from laptop to production.

**Open source, permissively licensed**: Apache 2.0 means you own your infrastructure. The managed service runs the same control plane you can run yourself. [Talk to an engineer →](https://www.rivet.dev/talk-to-an-engineer)

## Works with the tools you already use

Typed SDKs for your backend and React hooks for your frontend. Rivet runs on native Node.js and Bun with no custom runtime, so your existing packages, tooling, and tests work unchanged.

**Deploy nodes on**: [Vercel](https://www.rivet.dev/docs/deploy/self-host/workers/vercel/) • [Railway](https://www.rivet.dev/docs/deploy/self-host/workers/railway/) • [AWS ECS](https://www.rivet.dev/docs/deploy/self-host/workers/aws-ecs/) • [AWS Lambda](https://www.rivet.dev/docs/deploy/self-host/workers/aws-lambda/) • [GCP Cloud Run](https://www.rivet.dev/docs/deploy/self-host/workers/gcp-cloud-run/) • [Cloudflare](https://www.rivet.dev/docs/deploy/self-host/workers/cloudflare/) • [Kubernetes](https://www.rivet.dev/docs/deploy/self-host/workers/kubernetes/) • [Docker](https://www.rivet.dev/docs/deploy/self-host/workers/docker-compose/)

**Frameworks**: [React](https://www.rivet.dev/actors/docs/clients/react) • [Next.js](https://www.rivet.dev/actors/docs/quickstart/next-js) • [Hono](./examples/hono) • [Elysia](./examples/elysia) • [tRPC](./examples/trpc) • [Effect](./examples/ai-agent-effect)

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

Yes. Rivet Cloud is fully managed. It can run your Actors for you on [Rivet Compute](https://www.rivet.dev/docs/deploy/cloud/compute/), with preview deployments and auto-scaling in edge regions, or you can [bring your own nodes](https://www.rivet.dev/docs/deploy/self-host/workers/) and run Actors on your compute while Rivet Cloud provides the control plane and routing layer.

</details>

<details>
<summary><b>Can I self-host Rivet?</b></summary>

Yes. Rivet is fully self-hostable; see the [self-hosting docs](https://www.rivet.dev/docs/deploy/self-host/control-plane/). The control plane runs anywhere a container runs, with guides for Docker Compose, Kubernetes, VMs and bare metal, Hetzner, Railway, Render, and AWS ECS. For enterprise support with a self-hosted deployment, [contact us](https://www.rivet.dev/enterprise/).

</details>

<details>
<summary><b>Do I still need Postgres, FoundationDB, or NATS?</b></summary>

No. As of Rivet 3.0, an object storage bucket is the only dependency: there is no database to run and no message bus, and control plane instances find each other through the bucket. Rivet works with Amazon S3, Google Cloud Storage, Azure Blob Storage, and any S3-compatible store such as MinIO, Cloudflare R2, or Hetzner Object Storage. A local filesystem backend is available for trying Rivet on one machine.

</details>

<details>
<summary><b>Do reads and writes wait on S3?</b></summary>

No. A write is acknowledged once a majority of the instances holding that Actor's volume have it on disk; the bucket receives the durable copy in the background, shortly after. Reads are served from memory or from the instance's local disk. If the bucket becomes unavailable, writes and reads of recent data stay available; only reads of older Actors whose data has already moved to the bucket wait for it to come back.

</details>

<details>
<summary><b>What do I need to run to self-host?</b></summary>

Four things: control plane instances, a disk per instance, a bucket, and nodes. Every instance runs the same image and every service; run one to try Rivet and three or more in production. Each instance gets its own persistent block-storage disk for recent writes and its identity, never shared between instances. One bucket per cluster holds the durable copy of every Actor's data, and nodes run your Actors. There is no database to run and no message bus. Losing one instance out of three loses nothing: the other instances keep serving and rebuild the missing replicas.

</details>

<details>
<summary><b>What does the 3.0 preview mean, and do I need to change my code?</b></summary>

No code changes. Rivet 3.0 uses the same RivetKit SDK and protocol as 2.x; the upgrade is entirely in the storage engine. 3.0 is in preview because multi-region, serverless, and ACLs have not landed yet, and it is still being hardened under production load. Rivet 2.x stays supported while 3.0 is in preview, and a migration tool will move 2.x deployments to 3.0 with zero downtime.

</details>

<details>
<summary><b>Is Rivet open-source?</b></summary>

Yes. Rivet is open source under the permissive Apache 2.0 license. You're looking at it.

</details>

<details>
<summary><b>How does Rivet compare to Kubernetes?</b></summary>

Rivet is an orchestrator for stateful Actors. Think of an Actor like a pod, but much smaller, more lightweight, and faster to start: 72 KB and 6.9 ms versus 200 MB and seconds. Each Actor also comes with a bottomless SQLite database for structured persistence, instead of or in addition to a filesystem, which gives it more flexibility than a Kubernetes pod.

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
<summary><b>Does Rivet support Bun?</b></summary>

Yes. Rivet supports Bun in addition to Node.js.

</details>

<details>
<summary><b>Does Rivet support Effect?</b></summary>

Yes. Rivet Actors work with Effect; see the [Effect agent example](./examples/ai-agent-effect).

</details>

<details>
<summary><b>Is Rivet like Erlang, Akka, or Orleans?</b></summary>

Rivet implements the virtual actor pattern, which is closest to Orleans, but focuses on what modern workloads need: SQLite persistence instead of simple JSON stores, multi-region routing, and support for modern runtimes. Erlang and Akka do not provide virtual actors out of the box, though ecosystem packages exist that offer functionality comparable to Rivet Actors.

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
