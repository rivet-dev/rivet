import { DurableObject, env } from "cloudflare:workers";
import type { ExecutionContext } from "hono";
import invariant from "invariant";
import type { ActorKey, ActorRouter, Registry, RunConfig } from "rivetkit";
import { createActorRouter, createClientWithDriver } from "rivetkit";
import type { ActorDriver } from "rivetkit/driver-helpers";
import {
	type ManagerDriver,
	serializeEmptyPersistData,
} from "rivetkit/driver-helpers";
import { promiseWithResolvers } from "rivetkit/utils";
import {
	CloudflareDurableObjectGlobalState,
	createCloudflareActorsActorDriverBuilder,
} from "./actor-driver";
import type { Bindings } from "./handler";
import { logger } from "./log";

export const KEYS = {
	NAME: "rivetkit:name",
	KEY: "rivetkit:key",
	PERSIST_DATA: "rivetkit:data",
	CONN_PREFIX: "rivetkit:conn:",
};

// Helper to create a connection key for Cloudflare
export function makeCloudflareConnKey(connId: string): string {
	return `${KEYS.CONN_PREFIX}${connId}`;
}

export interface ActorHandlerInterface extends DurableObject {
	initialize(req: ActorInitRequest): Promise<void>;
}

export interface ActorInitRequest {
	name: string;
	key: ActorKey;
	input?: unknown;
}

interface InitializedData {
	name: string;
	key: ActorKey;
}

export type DurableObjectConstructor = new (
	...args: ConstructorParameters<typeof DurableObject<Bindings>>
) => DurableObject<Bindings>;

interface LoadedActor {
	actorRouter: ActorRouter;
	actorDriver: ActorDriver;
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

		async #loadActor(): Promise<LoadedActor> {
			// Wait for init
			if (!this.#initialized) {
				// Wait for init
				if (this.#initializedPromise) {
					await this.#initializedPromise.promise;
				} else {
					this.#initializedPromise = promiseWithResolvers();
					const res = await this.ctx.storage.get([
						KEYS.NAME,
						KEYS.KEY,
						KEYS.PERSIST_DATA,
					]);
					if (res.get(KEYS.PERSIST_DATA)) {
						const name = res.get(KEYS.NAME) as string;
						if (!name) throw new Error("missing actor name");
						const key = res.get(KEYS.KEY) as ActorKey;
						if (!key) throw new Error("missing actor key");

						logger().debug({
							msg: "already initialized",
							name,
							key,
						});

						this.#initialized = { name, key };
						this.#initializedPromise.resolve();
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

			// Save actor
			this.#actor = {
				actorRouter,
				actorDriver,
			};

			// Initialize the actor instance with proper metadata
			// This ensures the actor driver knows about this actor
			await actorDriver.loadActor(actorId);

			return this.#actor;
		}

		/** RPC called by the service that creates the DO to initialize it. */
		async initialize(req: ActorInitRequest) {
			// TODO: Need to add this to a core promise that needs to be resolved before start

			await this.ctx.storage.put({
				[KEYS.NAME]: req.name,
				[KEYS.KEY]: req.key,
				[KEYS.PERSIST_DATA]: serializeEmptyPersistData(req.input),
			});
			this.#initialized = {
				name: req.name,
				key: req.key,
			};

			logger().debug({ msg: "initialized actor", key: req.key });

			// Preemptively actor so the lifecycle hooks are called
			await this.#loadActor();
		}

		async fetch(request: Request): Promise<Response> {
			const { actorRouter } = await this.#loadActor();

			const actorId = this.ctx.id.toString();
			return await actorRouter.fetch(request, {
				actorId,
			});
		}

		async alarm(): Promise<void> {
			const { actorDriver } = await this.#loadActor();
			const actorId = this.ctx.id.toString();

			// Load the actor instance and trigger alarm
			const actor = await actorDriver.loadActor(actorId);
			await actor._onAlarm();
		}
	};
}
