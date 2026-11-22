# @rivetkit/effect-cluster

> **Experimental.** APIs and storage layout may change before a stable release.

Run [Effect Cluster](https://effect.website) entities, workflows, persisted queues, and singletons on Rivet Actors. Your Effect code stays the same; only the layer that provides `Sharding` changes.

Every entity instance is its own Rivet Actor, with its own SQLite database for its mailbox and state. There is no Postgres, no shard ring, and no storage polling.

## Why Rivet

- **No hash ring.** Effect's SQL backend hashes every entity id to one of 300 shards. Each runner computes the same hash ring and holds its shards with Postgres advisory locks. On Rivet, each entity is an actor that the gateway finds by its id, so there are no shards to assign, lock, or hand off during a deploy.
- **No single database to outgrow.** With the SQL backend, every message, reply, and workflow step from every runner goes through one Postgres primary, so its connections and write throughput cap the cluster. On Rivet, each entity writes to its own SQLite, so storage throughput grows with the number of entities.
- **Scales horizontally.** Add nodes and Rivet places new and waking actors on them. Nothing is rebalanced, and parallelism is not capped by a fixed shard count.
- **On-time scheduling, no polling.** The SQL backend finds delayed and unclaimed messages by polling Postgres every 10 seconds, so a `DeliverAt` message can run up to 10 seconds late. On Rivet, `DeliverAt` wakes the actor with a schedule at the requested time.
- **Idle entities cost nothing.** Idle actors sleep and wake on their next message or schedule. You can have millions of entities (one per user, document, or order) without keeping them in memory.
- **Per-entity SQLite.** `ActorSqlClient` gives each entity its own SQL database, which commits in the same transaction as its mailbox. You don't have to design, index, or migrate shared tables.
- **Cheaper long-term storage.** Rivet actor storage is about 10x cheaper than Postgres for long-term storage, so workflow history and entity state can grow without growing a database.
- **Nothing to operate.** There is no Postgres to provision, no connection pools to size, and no shared message tables to clean up. Destroying a workflow's actor deletes its history.
- **Managed hosting.** [Rivet Cloud](https://rivet.dev) runs your Effect cluster for you. You can also self-host the open-source Rivet Engine.

## How it works

![One Rivet Actor per entity](https://github.com/user-attachments/assets/4cf7233c-09f7-446f-b172-833f8b57c3b0)

| Effect Cluster | Rivet Actor |
| --- | --- |
| Entity instance | One actor named after the entity type, keyed by the entity id |
| Workflow execution | One `Workflow/<name>` actor per execution id |
| Persisted queue | One `effect.queue` actor per queue name |
| Singleton and `ClusterCron` | One `effect.singleton` actor per singleton |
| `DeliverAt`, durable clocks | Actor schedules, with a recurring heartbeat that replays missed work |
| `maxIdleTime` | The actor's sleep timeout |

A persisted ask is one action through the gateway. The actor stores the request, runs the handler, and stores the reply in its own SQLite before it responds:

![A persisted ask on Rivet](https://github.com/user-attachments/assets/185886ed-f982-412d-9a01-4e7eb37e0883)

## Compared with the SQL backend

Effect's SQL backend coordinates a fleet of runners through one Postgres database:

![Effect SQL backend on Postgres](https://github.com/user-attachments/assets/df90e4dd-2776-412d-bcaf-30bf33c1afeb)

The same persisted ask on each backend:

![The persisted ask on Postgres and on Rivet](https://github.com/user-attachments/assets/15293c63-da48-4e03-ae82-c32fe4b485e8)

| | SQL backend (Postgres) | Rivet |
| --- | --- | --- |
| Placement | Shard (300 per group) on a runner | Actor per entity |
| Ownership | Advisory locks and a hash ring each runner computes | Rivet runs one instance per key |
| You operate | Postgres 14+, a lock session per runner | Nothing on Rivet Cloud; the Rivet Engine if self-hosted |
| `DeliverAt` precision | About 10 s (poll interval) | Milliseconds (actor schedule) |
| Idle entities | In memory until `maxIdleTime` | Sleep after `maxIdleTime`, with no limit on count |
| Storage | One shared database | One SQLite database per entity |

## Usage

Host your entities on a Rivet node with `RivetCluster.serve`:

```ts
import { ActorSqlClient, RivetCluster } from "@rivetkit/effect-cluster";
import { Effect, Layer, Schema } from "effect";
import { ClusterSchema, Entity } from "effect/cluster";
import { Rpc } from "effect/rpc";

export const Counter = Entity.make("Counter", [
	Rpc.make("Increment", {
		payload: { op: Schema.String },
		primaryKey: ({ op }) => op,
		success: Schema.Number,
	}).annotate(ClusterSchema.Persisted, true),
]);

const CounterLive = Counter.toLayer(
	Effect.gen(function* () {
		// Each counter has its own SQLite database.
		const sql = yield* ActorSqlClient.orDie;
		yield* sql`CREATE TABLE IF NOT EXISTS counter (id INTEGER PRIMARY KEY, value INTEGER NOT NULL)`.pipe(
			Effect.orDie,
		);
		return Counter.of({
			Increment: () =>
				sql<{ readonly value: number }>`
					INSERT INTO counter (id, value) VALUES (1, 1)
					ON CONFLICT (id) DO UPDATE SET value = value + 1
					RETURNING value`.pipe(
					Effect.map((rows) => rows[0]!.value),
					Effect.orDie,
				),
		});
	}),
);

const ClusterLive = RivetCluster.serve(CounterLive, {
	entities: [Counter],
});

Layer.launch(ClusterLive).pipe(Effect.runFork);
```

Call entities from any process, such as an API server, with `RivetCluster.layer`:

```ts
const ClientLive = RivetCluster.layer({ entities: [Counter] });

const program = Effect.gen(function* () {
	const counter = (yield* Counter.client)("counter-1");
	return yield* counter.Increment({ op: "request-1" });
}).pipe(Effect.provide(ClientLive));
```

Both layers also provide `WorkflowEngine` and `PersistedQueueFactory`, so `effect/workflow` workflows and `PersistedQueue` run on Rivet Actors too. The RivetKit client and registry read `RIVET_ENDPOINT` and the other standard RivetKit settings; pass `client` and `registry` to override them.

## Options

| Option | Default | Description |
| --- | --- | --- |
| `entities` | | Every entity type this process calls or hosts |
| `client` | RivetKit defaults | RivetKit client configuration |
| `registry` | RivetKit defaults | RivetKit registry configuration (`serve` only); `use` adds your other actors |
| `keepAliveHeartbeat` | 30 s | How often a busy entity re-arms its recovery schedule; bounds how long it stays down after its node is lost |
| `actionTimeout` | 15 min | Upper bound of one entity action, and so of one ask |

## Current limitations

- An ask that runs longer than `actionTimeout` fails.
- `Sharding.getSnowflake` and `PersistedQueue` cleanup are not supported.
- Entity type names must fit Rivet's 64-byte actor name limit.
- `serve` raises RivetKit's message size limits to about 2 MB for the whole registry.

## License

Apache-2.0. Portions are adapted from [Effect](https://github.com/Effect-TS/effect) under the MIT License; see [`LICENSE-effect`](./LICENSE-effect).
