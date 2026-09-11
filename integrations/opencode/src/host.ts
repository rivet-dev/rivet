import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { ConfigPluginSource } from "@opencode/core/config/plugin/source";
import { Database } from "@opencode/core/database/database";
import { EffectDrizzleSqlite } from "@opencode/core/database/drizzle";
import { DatabaseMigration } from "@opencode/core/database/migration";
import { Environment } from "@opencode/core/environment/index";
import { Pty } from "@opencode/core/pty";
import { Snapshot } from "@opencode/core/snapshot";
import { PluginPromise } from "@opencode/core/plugin/promise";
import { SessionRestart } from "@opencode/core/session/execution/restart";
import { OpenCode } from "@opencode/sdk/effect";
import { Global } from "@opencode/util/global";
import { CrossSpawnSpawner } from "@opencode/util/cross-spawn-spawner";
import type { Sandbox } from "@rivet-dev/sandbox-adapter";
import { Effect, Exit, Layer, Scope, Stream } from "effect";
import { ChildProcessSpawner } from "effect/unstable/process/ChildProcessSpawner";
import { sandboxEnvironment } from "./sandbox.js";
import { sandboxProfile } from "./sandbox-profile.js";
import { type ActorSqlite, sqliteLayer } from "./sqlite.js";

export type OpenCodeOptions = Omit<
	OpenCode.CreateOptions,
	"database" | "events" | "fs" | "instances" | "workspaceProviders"
>;
export type OpenCodePlugin = Parameters<typeof PluginPromise.fromPromise>[0];
export type OpenCodeEvent = Stream.Success<
	ReturnType<OpenCode.Interface["event"]["subscribe"]>
>;

/** Owns one embedded SDK and closes every SDK fiber before releasing the sandbox. */
export async function createHost(
	database: ActorSqlite,
	options: OpenCodeOptions,
	sandbox?: Sandbox,
	plugins: readonly OpenCodePlugin[] = [],
) {
	const directory = await mkdtemp(join(tmpdir(), "rivet-opencode-"));
	const scope = await Effect.runPromise(Scope.make());
	const close = async () => {
		try {
			await Effect.runPromise(Scope.close(scope, Exit.void));
		} finally {
			await rm(directory, { recursive: true, force: true });
		}
	};
	try {
		const global = Global.layerWith(
			Object.fromEntries(
				[
					"home",
					"data",
					"cache",
					"config",
					"state",
					"tmp",
					"bin",
					"log",
					"repos",
				].map((key) => [key, join(directory, key)]),
			),
		);
		// Rivet owns journal mode, durability, and connection lifecycle. Use the
		// upstream schema/migrations without the Node profile's tuning PRAGMAs.
		const databaseLayer = Layer.effect(
			Database.Service,
			Effect.gen(function* () {
				const db = yield* EffectDrizzleSqlite.makeWithDefaults();
				yield* db.run("PRAGMA foreign_keys = ON");
				yield* DatabaseMigration.apply(db);
				return { db };
			}).pipe(Effect.orDie),
		).pipe(Layer.provide(sqliteLayer(database)), Layer.provide(global));
		let resume = (): Promise<void> =>
			Promise.reject(
				new Error("OpenCode recovery service was not initialized"),
			);
		const recovery = SessionRestart.node.mapLayer((layer) =>
			Layer.effect(
				SessionRestart.Service,
				Effect.map(SessionRestart.Service, (service) => {
					resume = () => Effect.runPromise(service.resumeSuspendedSessions);
					// The SDK starts recovery during create(). Defer it until plugins and
					// the actor's event/keepalive subscription have been installed.
					return { resumeSuspendedSessions: Effect.void };
				}),
			).pipe(Layer.provide(layer)),
		);
		const overrides = [
			Global.node.replace(global),
			Database.node.replace(databaseLayer),
			SessionRestart.node.replace(recovery),
		];
		if (sandbox) {
			const environment = sandboxEnvironment(sandbox);
			overrides.push(
				...sandboxProfile(),
				Environment.node.replace(
					Layer.succeed(Environment.Service, environment),
				),
				CrossSpawnSpawner.node.replace(
					Layer.succeed(ChildProcessSpawner, environment.spawner),
				),
				Snapshot.node.replace(Snapshot.noopLayer),
				ConfigPluginSource.node.replace(ConfigPluginSource.empty),
				Pty.node.replace(
					Layer.succeed(
						Pty.Service,
						Pty.Service.of({
							list: () => Effect.succeed([]),
							create: () =>
								Effect.die(
									new Error(
										"Interactive PTYs are unavailable through sandbox-adapter; use session.shell",
									),
								),
							get: (ptyID) => Effect.fail(new Pty.NotFoundError({ ptyID })),
							update: (ptyID) => Effect.fail(new Pty.NotFoundError({ ptyID })),
							remove: (ptyID) => Effect.fail(new Pty.NotFoundError({ ptyID })),
							write: (ptyID) => Effect.fail(new Pty.NotFoundError({ ptyID })),
							attach: (ptyID) => Effect.fail(new Pty.NotFoundError({ ptyID })),
						}),
					),
				),
			);
		}
		const client = await Effect.runPromise(
			OpenCode.create(
				{
					...options,
					config: { ...options.config, ...(sandbox ? { project: false } : {}) },
					events: { persist: true },
					fs: { filewatcher: false, fff: false },
				},
				{ overrides },
			).pipe(Scope.provide(scope)),
		);
		for (const plugin of plugins)
			await Effect.runPromise(client.plugin(PluginPromise.fromPromise(plugin)));
		return { client, scope, close, resume };
	} catch (error) {
		await close();
		throw error;
	}
}

export type OpenCodeHost = Awaited<ReturnType<typeof createHost>>;
