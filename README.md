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
    12 ms cold starts, 72 KB per Actor, billions on one control plane.<br/>
    Open source and self-hostable.
  </p>
  <p>
    <a href="https://www.rivet.dev/docs">Quickstart</a> •
    <a href="https://www.rivet.dev/actors/docs">Documentation</a> •
    <a href="https://www.rivet.dev/changelog">Changelog</a> •
    <a href="https://www.rivet.dev/discord">Discord</a> •
    <a href="https://x.com/rivet_dev">X</a>
  </p>
</div>

## What is Rivet?

Rivet is an orchestrator for agentic workloads. Where Kubernetes schedules pods, Rivet schedules Actors: long-lived processes with durable state, a SQLite database, a queue, realtime connections, and scheduling, each addressed by key. An Actor starts in 12 ms, weighs 72 KB, and hibernates when idle, so one control plane runs a thousand agents or a billion.

Create one Actor per agent, per session, per user, or per tenant. Run it on plain Node.js, Bun, or Rust. Open source and self-hostable.

- **Run 2,829× more workloads per server**: An Actor lives inside a worker process you already run, not in its own container or VM. A server that held hundreds of pods holds hundreds of thousands of Actors.
- **Start in milliseconds, not minutes**: A cold start includes scheduling the Actor, loading its state, and serving the first request. No image pull, no container boot.
- **Built for scale**: Actors are scheduled independently, so adding capacity means adding machines. The same control plane runs a thousand Actors or a billion.
- **Hibernates when idle, wakes on demand**: An idle Actor writes its state and unloads. The next request brings it back in 12 ms with nothing lost.
- **Durable state for every workload**: Every Actor gets a SQLite database and a POSIX filesystem, tiered to S3. Idle Actors cost nothing, so millions can sit parked with their state intact.
- **Run code agents generate at runtime**: Each piece of new, untrusted code gets an Actor of its own, with its own database and filesystem and no reach into anyone else's.
- **Works with the tools you already use**: Typed SDKs for your backend and React hooks for your frontend. No custom runtime, so your existing packages, tooling, and tests work unchanged.

**Backend**

```typescript
const agent = actor({
  // In-memory, persisted state for the Actor
  state: { messages: [] as Message[] },

  // Long-running Actor process
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
// Connect to an Actor
const agent = client.agent.getOrCreate("agent-123").connect();

// Listen for realtime events
agent.on("token", delta => process.stdout.write(delta));

// Send message to Actor
await agent.queue.send("how many r's in strawberry?");
```

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
- [Cloudflare Workers](https://www.rivet.dev/actors/docs/quickstart/cloudflare)

[View documentation →](https://www.rivet.dev/docs)

</td>
</tr>
</table>

## Whatever the workload, there's an Actor for it

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
| **Cold start** | **12.3 ms** | ~6 s |
| **Memory per instance** | **72.4 KB** | ~200 MB |
| **Memory while idle** | **0 MB** (hibernates) | Always resident |
| **Scale** | **Billions** of Actors on one control plane | ~5k nodes per cluster |

<details>
<summary>Benchmark details & methodology</summary>

- **Cold start (12.3 ms):** Round trip of an HTTP request waking an Actor from hibernation: 12.3 ms p50, 27.7 ms p99. Measured September 2026 on FoundationDB with the Rust rivetkit SDK. Uses an experimental Rivet feature that will be enabled by default in an upcoming update.
- **Memory per instance (72.4 KB):** RSS increase per running Actor. Measured September 2026 on FoundationDB with the Rust rivetkit SDK.
- **Kubernetes figures** are typical published values, not a matched benchmark. ~6 s cold start assumes a pre-provisioned node; ~200 MB is a typical idle Node.js pod; ~5k nodes is the documented cluster limit.

</details>

## Features

Routing, scheduling, types, and telemetry, built in.

- **Fault tolerance by design**: A worker going down reschedules its Actors elsewhere with their state intact. No replay harness or checkpointing of your own.
- **Multi-region**: Place Actors near the users and data they serve, and route requests to wherever each one currently lives.
- **HTTP & WebSocket networking**: Address an Actor directly over HTTP or hold a live WebSocket to it. No queue or broker in between.
- **End-to-end type safety**: Actor definitions generate their own client types, so a signature change breaks the build rather than production.
- **React SDK**: First-party hooks that subscribe a component to an Actor's state and keep it live as the Actor updates.
- **OpenTelemetry & observability**: Traces, metrics, and structured logs emitted in OTel format, into the collector you already run.
- **Single Rust binary**: The control plane ships as one static binary with no external dependencies to stand up first.
- **Cron & scheduling**: Wake an Actor on a schedule or at a timestamp it sets for itself, without a separate scheduler.
- **Sleeps when idle**: Idle Actors release their resources and wake with durable state intact when the next request arrives.
- **Actor-to-Actor calls**: Actors address each other by key and call across the cluster as if the other one were local.
- **No Kubernetes operator**: One control plane behind a load balancer, speaking plain HTTP inside your VPC. No CRDs, no operator, no service mesh to keep alive.
- **Open source**: Apache 2.0, self-hostable in full. The managed service runs the same control plane you can run yourself.

## Start local. Deploy when ready.

<table>
<tr>
<td width="33%" valign="top">

### Local & Self-Host

Install Actors and run it locally while you build. When you ship, run the same open-source control plane as a Rust binary or container on your own infrastructure.

```bash
npm install rivetkit
```

[Open the quickstart →](https://www.rivet.dev/actors/docs/quickstart/backend)
[Read self-hosting docs →](https://www.rivet.dev/docs/deploy/self-host/control-plane/)

</td>
<td width="33%" valign="top">

### Rivet Cloud

Deploy Actors on Rivet Cloud with managed infrastructure and persisted Actor data. Or bring your own worker and run Actors on your compute while Rivet Cloud provides the control plane and routing layer.

[Open the dashboard →](https://dashboard.rivet.dev)

</td>
<td width="33%" valign="top">

### BYOC

Run the control plane inside your own VPC, fully managed by Rivet. Your data never leaves your cloud and there is no inbound management connection.

[Read the BYOC docs →](https://www.rivet.dev/docs/deploy/byoc/)

</td>
</tr>
</table>

**Run your agent infrastructure where your data already lives.** The control plane is a single Rust binary: one process to install, monitor, and upgrade, on Kubernetes with the Enterprise Helm chart or as a systemd unit. Use Postgres or FoundationDB for persistence and tiered storage to S3. Local dev matches production: the same binary, APIs, and control-plane behavior from laptop to production.

**Open source, permissively licensed**: Apache 2.0 means you own your infrastructure. The managed service runs the same control plane you can run yourself. [Talk to an engineer →](https://www.rivet.dev/talk-to-an-engineer)

## Works with the tools you already use

Typed SDKs for your backend and React hooks for your frontend. Rivet runs on native Node.js and Bun with no custom runtime, so your existing packages, tooling, and tests work unchanged.

**Deploy workers on**: [Vercel](https://www.rivet.dev/docs/deploy/self-host/workers/vercel/) • [Railway](https://www.rivet.dev/docs/deploy/self-host/workers/railway/) • [AWS ECS](https://www.rivet.dev/docs/deploy/self-host/workers/aws-ecs/) • [AWS Lambda](https://www.rivet.dev/docs/deploy/self-host/workers/aws-lambda/) • [GCP Cloud Run](https://www.rivet.dev/docs/deploy/self-host/workers/gcp-cloud-run/) • [Cloudflare](https://www.rivet.dev/docs/deploy/self-host/workers/cloudflare/) • [Kubernetes](https://www.rivet.dev/docs/deploy/self-host/workers/kubernetes/) • [Docker](https://www.rivet.dev/docs/deploy/self-host/workers/docker-compose/)

**Frameworks**: [React](https://www.rivet.dev/actors/docs/clients/react) • [Next.js](https://www.rivet.dev/actors/docs/quickstart/next-js) • [Hono](./examples/hono) • [Elysia](./examples/elysia) • [tRPC](./examples/trpc) • [Effect](./examples/ai-agent-effect)

**Runtimes**: [Node.js](https://www.rivet.dev/actors/docs/quickstart/backend) • [Bun](https://www.rivet.dev/actors/docs/quickstart/backend) • [Rust](https://www.rivet.dev/actors/docs/quickstart/rust)

**Tools**: [Vitest](https://www.rivet.dev/actors/docs/testing) • [OpenTelemetry](https://www.rivet.dev/actors/docs/general/tracing) • [Pino](https://www.rivet.dev/actors/docs/general/logging) • [AI SDK](./examples/ai-agent) • [OpenAPI](./rivetkit-openapi) • [AsyncAPI](./rivetkit-asyncapi)

[Request an integration →](https://github.com/rivet-dev/actors/issues/new)

## FAQ

<details>
<summary><b>Does Rivet have BYOC?</b></summary>

Yes. Rivet deploys into your own cloud account, including air-gapped environments: the control plane and storage run inside your VPC while Rivet operates them. See the [BYOC docs](https://www.rivet.dev/docs/deploy/byoc/).

</details>

<details>
<summary><b>Does Rivet have a cloud?</b></summary>

Yes. Rivet Cloud is fully managed. It can run your Actors for you on [Rivet Compute](https://www.rivet.dev/docs/deploy/cloud/compute/), with preview deployments and auto-scaling in edge regions, or you can [bring your own worker](https://www.rivet.dev/docs/deploy/self-host/workers/) and run Actors on your compute while Rivet Cloud provides the control plane and routing layer.

</details>

<details>
<summary><b>Can I self-host Rivet?</b></summary>

Yes. Rivet is fully self-hostable; see the [self-hosting docs](https://www.rivet.dev/docs/deploy/self-host/control-plane/). For enterprise support with a self-hosted deployment, [contact us](https://www.rivet.dev/enterprise/).

</details>

<details>
<summary><b>Is Rivet open-source?</b></summary>

Yes. Rivet is open source under the permissive Apache 2.0 license. You're looking at it.

</details>

<details>
<summary><b>How does Rivet compare to Kubernetes?</b></summary>

Rivet is an orchestrator for stateful Actors. Think of an Actor like a pod, but much smaller, more lightweight, and faster to start. Each Actor also comes with SQLite for structured persistence, instead of or in addition to a filesystem, which gives it more flexibility than a Kubernetes pod.

</details>

<details>
<summary><b>How does Rivet compare to Cloudflare Durable Objects?</b></summary>

Rivet is often used as an open-source alternative to Cloudflare Durable Objects that is easy to self-host and scales. It also fixes many of the constraints of Durable Objects: no 128 MB memory cap, no 10 GB SQLite limit, Actors are not randomly evicted, and you control the runtime because Actors run on vanilla Node.js or Bun instead of a custom runtime. Rivet also adds built-in connection handling, event broadcasting, durable scheduling and cron, full Vitest support, lifecycle hooks for draining, and control over Actor upgrades. See the [Cloudflare Durable Objects comparison](https://www.rivet.dev/actors/compare/rivet-actors-vs-cloudflare-durable-objects/).

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
| [Container Runner](./container-runner) | Runs Actors as containers |
| [Dashboard](./frontend) | Inspector for debugging Actors |
| [Documentation](./docs) | Source for [rivet.dev/actors/docs](https://www.rivet.dev/actors/docs) |
| [Examples](./examples) | Runnable examples |

## Community

- [Discord](https://www.rivet.dev/discord) - Chat with the community
- [X/Twitter](https://x.com/rivet_dev) - Follow for updates
- [Bluesky](https://bsky.app/profile/www.rivet.dev) - Follow for updates
- [GitHub Discussions](https://github.com/rivet-dev/actors/discussions) - Ask questions
- [GitHub Issues](https://github.com/rivet-dev/actors/issues) - Report bugs
- [Talk to an engineer](https://www.rivet.dev/talk-to-an-engineer) - Discuss your use case

## License

[Apache 2.0](LICENSE)
