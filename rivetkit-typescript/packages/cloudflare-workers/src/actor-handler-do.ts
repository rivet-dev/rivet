import { DurableObject, env } from "cloudflare:workers";
import type { ExecutionContext } from "hono";
import invariant from "invariant";
import type { ActorKey, ActorRouter, Registry, RunConfig } from "rivetkit";
import { createActorRouter, createClientWithDriver } from "rivetkit";
import type { ActorDriver, ManagerDriver } from "rivetkit/driver-helpers";
import { serializeEmptyPersistData } from "rivetkit/driver-helpers";
import { promiseWithResolvers, stringifyError } from "rivetkit/utils";
import {
	CloudflareDurableObjectGlobalState,
	createCloudflareActorsActorDriverBuilder,
} from "./actor-driver";
import { buildActorId, parseActorId } from "./actor-id";
import { GLOBAL_KV_KEYS } from "./global_kv";
import type { Bindings } from "./handler";
import { getCloudflareAmbientEnv } from "./handler";
import { kvPut } from "./kv_query";
import { logger } from "./log";

// KV keys used by the actor instance
const KEYS = {
	PERSIST_DATA: Uint8Array.from([1]),
};

export interface ActorHandlerInterface extends DurableObject {
	create(
		req: ActorInitRequest & { allowExisting?: boolean },
	): Promise<
		| { success: { actorId: string; created: boolean } }
		| { error: { actorAlreadyExists: true } }
	>;
	getMetadata(): Promise<
		| {
				actorId: string;
				name: string;
				key: ActorKey;
				destroying: boolean;
		  }
		| undefined
	>;
}

export interface ActorInitRequest {
	name: string;
	key: ActorKey;
	input?: unknown;
}

interface InitializedData {
	name: string;
	key: ActorKey;
	generation: number;
}

export type DurableObjectConstructor = new (
	...args: ConstructorParameters<typeof DurableObject<Bindings>>
) => DurableObject<Bindings>;

interface LoadedActor {
	actorRouter: ActorRouter;
	actorDriver: ActorDriver;
	generation: number;
}

export function createActorDurableObject(
	registry: Registry<any>,
	rootRunConfig: RunConfig,
): DurableObjectConstructor {
	const globalState = new CloudflareDurableObjectGlobalState();

	// Configure to use the runner role instead of server role
	const runConfig = Object.assign({}, rootRunConfig, { role: "runner" });

	/**
	 * Startup steps:
	 * 1. If not already created call `initialize`, otherwise check KV to ensure it's initialized
	 * 2. Load actor
	 * 3. Start service requests
	 */
	return class ActorHandler
		extends DurableObject<Bindings>
		implements ActorHandlerInterface
	{
		#initialized?: InitializedData;
		#initializedPromise?: ReturnType<typeof promiseWithResolvers<void>>;

		#actor?: LoadedActor;

		constructor(
			...args: ConstructorParameters<typeof DurableObject<Bindings>>
		) {
			super(...args);

			// Initialize SQL table for key-value storage
			//
			// We do this instead of using the native KV storage so we can store blob keys. The native CF KV API only supports string keys.
			this.ctx.storage.sql.exec(`
				CREATE TABLE IF NOT EXISTS _rivetkit_kv_storage(
					key BLOB PRIMARY KEY,
					value BLOB
				);
			`);

			// Initialize SQL table for actor metadata
			//
			// id always equals 1 in order to ensure that there's always exactly 1 row in this table
			this.ctx.storage.sql.exec(`
				CREATE TABLE IF NOT EXISTS _rivetkit_metadata(
					id INTEGER PRIMARY KEY CHECK (id = 1),
					name TEXT NOT NULL,
					key TEXT NOT NULL,
					destroyed INTEGER DEFAULT 0,
					generation INTEGER DEFAULT 0
				);
			`);
		}

		async #loadActor(): Promise<LoadedActor> {
			// Wait for init
			if (!this.#initialized) {
				// Wait for init
				if (this.#initializedPromise) {
					await this.#initializedPromise.promise;
				} else {
					this.#initializedPromise = promiseWithResolvers();

					// Query SQL for initialization data
					const cursor = this.ctx.storage.sql.exec(
						"SELECT name, key, destroyed, generation FROM _rivetkit_metadata WHERE id = 1",
					);
					const result = cursor.raw().next();

					if (!result.done && result.value) {
						const name = result.value[0] as string;
						const key = JSON.parse(
							result.value[1] as string,
						) as ActorKey;
						const destroyed = result.value[2] as number;
						const generation = result.value[3] as number;

						// Only initialize if not destroyed
						if (!destroyed) {
							logger().debug({
								msg: "already initialized",
								name,
								key,
								generation,
							});

							this.#initialized = { name, key, generation };
							this.#initializedPromise.resolve();
						} else {
							logger().debug(
								"actor is destroyed, waiting to re-initialize",
							);
						}
					} else {
						logger().debug("waiting to initialize");
					}
				}
			}

			// Check if already loaded
			if (this.#actor) {
				return this.#actor;
			}

			if (!this.#initialized) throw new Error("Not initialized");

			// Register DO with global state first
			// HACK: This leaks the DO context, but DO does not provide a native way
			// of knowing when the DO shuts down. We're making a broad assumption
			// that DO will boot a new isolate frequenlty enough that this is not an issue.
			const actorId = this.ctx.id.toString();
			globalState.setDOState(actorId, { ctx: this.ctx, env: env });

			// Configure actor driver
			invariant(runConfig.driver, "runConfig.driver");
			runConfig.driver.actor =
				createCloudflareActorsActorDriverBuilder(globalState);

			// Create manager driver (we need this for the actor router)
			const managerDriver = runConfig.driver.manager(
				registry.config,
				runConfig,
			);

			// Create inline client
			const inlineClient = createClientWithDriver(
				managerDriver,
				runConfig,
			);

			// Create actor driver
			const actorDriver = runConfig.driver.actor(
				registry.config,
				runConfig,
				managerDriver,
				inlineClient,
			);

			// Create actor router
			const actorRouter = createActorRouter(
				runConfig,
				actorDriver,
				false,
			);

			// Save actor with generation
			this.#actor = {
				actorRouter,
				actorDriver,
				generation: this.#initialized.generation,
			};

			// Build actor ID with generation for loading
			const actorIdWithGen = buildActorId(
				actorId,
				this.#initialized.generation,
			);

			// Initialize the actor instance with proper metadata
			// This ensures the actor driver knows about this actor
			await actorDriver.loadActor(actorIdWithGen);

			return this.#actor;
		}

		/** RPC called to get actor metadata without creating it */
		async getMetadata(): Promise<
			| {
					actorId: string;
					name: string;
					key: ActorKey;
					destroying: boolean;
			  }
			| undefined
		> {
			// Query the metadata
			const cursor = this.ctx.storage.sql.exec(
				"SELECT name, key, destroyed, generation FROM _rivetkit_metadata WHERE id = 1",
			);
			const result = cursor.raw().next();

			if (!result.done && result.value) {
				const name = result.value[0] as string;
				const key = JSON.parse(result.value[1] as string) as ActorKey;
				const destroyed = result.value[2] as number;
				const generation = result.value[3] as number;

				// Check if destroyed
				if (destroyed) {
					logger().debug({
						msg: "getMetadata: actor is destroyed",
						name,
						key,
						generation,
					});
					return undefined;
				}

				// Build actor ID with generation
				const doId = this.ctx.id.toString();
				const actorId = buildActorId(doId, generation);
				const destroying =
					globalState.actors.get(doId)?.destroying ?? false;

				logger().debug({
					msg: "getMetadata: found actor metadata",
					actorId,
					name,
					key,
					generation,
					destroying,
				});

				return { actorId, name, key, destroying };
			}

			logger().debug({
				msg: "getMetadata: no metadata found",
			});
			return undefined;
		}

		/** RPC called by the manager to create a DO. Can optionally allow existing actors. */
		async create(
			req: ActorInitRequest & { allowExisting?: boolean },
		): Promise<
			| { success: { actorId: string; created: boolean } }
			| { error: { actorAlreadyExists: true } }
		> {
			// Check if actor exists
			const checkCursor = this.ctx.storage.sql.exec(
				"SELECT destroyed, generation FROM _rivetkit_metadata WHERE id = 1",
			);
			const checkResult = checkCursor.raw().next();

			let created = false;
			let generation = 0;

			if (!checkResult.done && checkResult.value) {
				const destroyed = checkResult.value[0] as number;
				generation = checkResult.value[1] as number;

				if (!destroyed) {
					// Actor exists and is not destroyed
					if (!req.allowExisting) {
						// Fail if not allowing existing actors
						logger().debug({
							msg: "create failed: actor already exists",
							name: req.name,
							key: req.key,
							generation,
						});
						return { error: { actorAlreadyExists: true } };
					}

					// Return existing actor
					logger().debug({
						msg: "actor already exists",
						key: req.key,
						generation,
					});
					const doId = this.ctx.id.toString();
					const actorId = buildActorId(doId, generation);
					return { success: { actorId, created: false } };
				}

				// Actor exists but is destroyed - resurrect with incremented generation
				generation = generation + 1;
				created = true;
				logger().debug({
					msg: "resurrecting destroyed actor",
					key: req.key,
					oldGeneration: generation - 1,
					newGeneration: generation,
				});
			} else {
				// No actor exists - will create with generation 0
				generation = 0;
				created = true;
				logger().debug({
					msg: "creating new actor",
					key: req.key,
					generation,
				});
			}

			// Perform upsert - either inserts new or updates destroyed actor
			this.ctx.storage.sql.exec(
				`INSERT INTO _rivetkit_metadata (id, name, key, destroyed, generation)
				VALUES (1, ?, ?, 0, ?)
				ON CONFLICT(id) DO UPDATE SET
					name = excluded.name,
					key = excluded.key,
					destroyed = 0,
					generation = excluded.generation`,
				req.name,
				JSON.stringify(req.key),
				generation,
			);

			this.#initialized = {
				name: req.name,
				key: req.key,
				generation,
			};

			// Build actor ID with generation
			const doId = this.ctx.id.toString();
			const actorId = buildActorId(doId, generation);

			// Initialize storage and update KV when created or resurrected
			if (created) {
				// Initialize persist data in KV storage
				initializeActorKvStorage(this.ctx.storage.sql, req.input);

				// Update metadata in the background
				const env = getCloudflareAmbientEnv();
				const actorData = { name: req.name, key: req.key, generation };
				this.ctx.waitUntil(
					env.ACTOR_KV.put(
						GLOBAL_KV_KEYS.actorMetadata(actorId),
						JSON.stringify(actorData),
					),
				);
			}

			// Preemptively load actor so the lifecycle hooks are called
			await this.#loadActor();

			logger().debug({
				msg: created ? "actor created/resurrected" : "returning existing actor",
				actorId,
				created,
				generation,
			});

			return { success: { actorId, created } };
		}

		async fetch(request: Request): Promise<Response> {
			const { actorRouter, generation } = await this.#loadActor();

			// Build actor ID with generation
			const doId = this.ctx.id.toString();
			const actorId = buildActorId(doId, generation);

			return await actorRouter.fetch(request, {
				actorId,
			});
		}

		async alarm(): Promise<void> {
			const { actorDriver, generation } = await this.#loadActor();

			// Build actor ID with generation
			const doId = this.ctx.id.toString();
			const actorId = buildActorId(doId, generation);

			// Load the actor instance and trigger alarm
			const actor = await actorDriver.loadActor(actorId);
			await actor.onAlarm();
		}
	};
}

/**
 * Initialize KV storage with data needed for a new actor. We do this
 * separately since we don't have access to an ActorDriver yet.
 **/
function initializeActorKvStorage(
	sql: SqlStorage,
	input: unknown | undefined,
): void {
	const persistData = serializeEmptyPersistData(input);
	kvPut(sql, KEYS.PERSIST_DATA, persistData);
}
