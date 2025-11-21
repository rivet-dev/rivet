import type { Context as HonoContext } from "hono";
import invariant from "invariant";
import { ActorStopping } from "@/actor/errors";
import { type ActorRouter, createActorRouter } from "@/actor/router";
import { routeWebSocket } from "@/actor/router-websocket-endpoints";
import { createClientWithDriver } from "@/client/client";
import { ClientConfigSchema } from "@/client/config";
import { InlineWebSocketAdapter } from "@/common/inline-websocket-adapter";
import { noopNext } from "@/common/utils";
import type {
	ActorDriver,
	ActorOutput,
	CreateInput,
	GetForIdInput,
	GetOrCreateWithKeyInput,
	GetWithKeyInput,
	ListActorsInput,
	ManagerDriver,
} from "@/driver-helpers/mod";
import { ManagerInspector } from "@/inspector/manager";
import { type Actor, ActorFeature, type ActorId } from "@/inspector/mod";
import type { ManagerDisplayInformation } from "@/manager/driver";
import type {
	DriverConfig,
	Encoding,
	RegistryConfig,
	RunConfig,
	UniversalWebSocket,
} from "@/mod";
import type * as schema from "@/schemas/file-system-driver/mod";
import type { FileSystemGlobalState } from "./global-state";
import { logger } from "./log";
import { generateActorId } from "./utils";

export class FileSystemManagerDriver implements ManagerDriver {
	#registryConfig: RegistryConfig;
	#runConfig: RunConfig;
	#state: FileSystemGlobalState;
	#driverConfig: DriverConfig;

	#actorDriver: ActorDriver;
	#actorRouter: ActorRouter;

	inspector?: ManagerInspector;

	constructor(
		registryConfig: RegistryConfig,
		runConfig: RunConfig,
		state: FileSystemGlobalState,
		driverConfig: DriverConfig,
	) {
		this.#registryConfig = registryConfig;
		this.#runConfig = runConfig;
		this.#state = state;
		this.#driverConfig = driverConfig;

		if (runConfig.inspector.enabled) {
			const startedAt = new Date().toISOString();
			function transformActor(actorState: schema.ActorState): Actor {
				return {
					id: actorState.actorId as ActorId,
					name: actorState.name,
					key: actorState.key as string[],
					startedAt: startedAt,
					createdAt: new Date(
						Number(actorState.createdAt),
					).toISOString(),
					features: [
						ActorFeature.State,
						ActorFeature.Connections,
						ActorFeature.Console,
						ActorFeature.EventsMonitoring,
						ActorFeature.Database,
					],
				};
			}

			this.inspector = new ManagerInspector(() => {
				return {
					getAllActors: async ({ cursor, limit }) => {
						const itr = this.#state.getActorsIterator({ cursor });
						const actors: Actor[] = [];

						for await (const actor of itr) {
							actors.push(transformActor(actor));
							if (limit && actors.length >= limit) {
								break;
							}
						}
						return actors;
					},
					getActorById: async (id) => {
						try {
							const result =
								await this.#state.loadActorStateOrError(id);
							return transformActor(result);
						} catch {
							return null;
						}
					},
					getBuilds: async () => {
						return Object.keys(this.#registryConfig.use).map(
							(name) => ({
								name,
							}),
						);
					},
					createActor: async (input) => {
						const { actorId } = await this.createActor(input);
						try {
							const result =
								await this.#state.loadActorStateOrError(
									actorId,
								);
							return transformActor(result);
						} catch {
							return null;
						}
					},
				};
			});
		}

		// Actors run on the same node as the manager, so we create a dummy actor router that we route requests to
		const inlineClient = createClientWithDriver(
			this,
			ClientConfigSchema.parse({}),
		);
		this.#actorDriver = this.#driverConfig.actor(
			registryConfig,
			runConfig,
			this,
			inlineClient,
		);
		this.#actorRouter = createActorRouter(
			this.#runConfig,
			this.#actorDriver,
			registryConfig.test.enabled,
		);
	}

	async sendRequest(
		actorId: string,
		actorRequest: Request,
	): Promise<Response> {
		return await this.#actorRouter.fetch(actorRequest, {
			actorId,
		});
	}

	async openWebSocket(
		path: string,
		actorId: string,
		encoding: Encoding,
		params: unknown,
	): Promise<UniversalWebSocket> {
		// Handle raw WebSocket paths
		const pathOnly = path.split("?")[0];
		const normalizedPath = pathOnly.startsWith("/")
			? pathOnly
			: `/${pathOnly}`;
		const wsHandler = await routeWebSocket(
			// TODO: Create fake request
			undefined,
			normalizedPath,
			{},
			this.#runConfig,
			this.#actorDriver,
			actorId,
			encoding,
			params,
			undefined,
			undefined,
			false,
			false,
		);
		return new InlineWebSocketAdapter(wsHandler);
	}

	async proxyRequest(
		c: HonoContext,
		actorRequest: Request,
		actorId: string,
	): Promise<Response> {
		return await this.#actorRouter.fetch(actorRequest, {
			actorId,
		});
	}

	async proxyWebSocket(
		c: HonoContext,
		path: string,
		actorId: string,
		encoding: Encoding,
		params: unknown,
	): Promise<Response> {
		const upgradeWebSocket = this.#runConfig.getUpgradeWebSocket?.();
		invariant(upgradeWebSocket, "missing getUpgradeWebSocket");

		// Handle raw WebSocket paths
		const pathOnly = path.split("?")[0];
		const normalizedPath = pathOnly.startsWith("/")
			? pathOnly
			: `/${pathOnly}`;
		const wsHandler = await routeWebSocket(
			// TODO: Create new request with new path
			c.req.raw,
			normalizedPath,
			c.req.header(),
			this.#runConfig,
			this.#actorDriver,
			actorId,
			encoding,
			params,
			undefined,
			undefined,
			false,
			false,
		);
		return upgradeWebSocket(() => wsHandler)(c, noopNext());
	}

	async getForId({
		actorId,
	}: GetForIdInput): Promise<ActorOutput | undefined> {
		// Validate the actor exists
		const actor = await this.#state.loadActor(actorId);
		if (!actor.state) {
			return undefined;
		}
		if (this.#state.isActorStopping(actorId)) {
			throw new ActorStopping(actorId);
		}

		try {
			// Load actor state
			return {
				actorId,
				name: actor.state.name,
				key: actor.state.key as string[],
			};
		} catch (error) {
			logger().error({
				msg: "failed to read actor state",
				actorId,
				error,
			});
			return undefined;
		}
	}

	async getWithKey({
		name,
		key,
	}: GetWithKeyInput): Promise<ActorOutput | undefined> {
		// Generate the deterministic actor ID
		const actorId = generateActorId(name, key);

		// Check if actor exists
		const actor = await this.#state.loadActor(actorId);
		if (actor.state) {
			return {
				actorId,
				name,
				key,
			};
		}

		return undefined;
	}

	async getOrCreateWithKey(
		input: GetOrCreateWithKeyInput,
	): Promise<ActorOutput> {
		// Generate the deterministic actor ID
		const actorId = generateActorId(input.name, input.key);

		// Use the atomic getOrCreateActor method
		const actorEntry = await this.#state.loadOrCreateActor(
			actorId,
			input.name,
			input.key,
			input.input,
		);
		invariant(actorEntry.state, "must have state");

		return {
			actorId: actorEntry.state.actorId,
			name: actorEntry.state.name,
			key: actorEntry.state.key as string[],
		};
	}

	async createActor({ name, key, input }: CreateInput): Promise<ActorOutput> {
		// Generate the deterministic actor ID
		const actorId = generateActorId(name, key);

		await this.#state.createActor(actorId, name, key, input);

		return {
			actorId,
			name,
			key,
		};
	}

	async listActors({ name }: ListActorsInput): Promise<ActorOutput[]> {
		const actors: ActorOutput[] = [];
		const itr = this.#state.getActorsIterator({});

		for await (const actor of itr) {
			if (actor.name === name) {
				actors.push({
					actorId: actor.actorId,
					name: actor.name,
					key: actor.key as string[],
					createTs: Number(actor.createdAt),
				});
			}
		}

		// Sort by create ts desc (most recent first)
		actors.sort((a, b) => {
			const aTs = a.createTs ?? 0;
			const bTs = b.createTs ?? 0;
			return bTs - aTs;
		});

		return actors;
	}

	displayInformation(): ManagerDisplayInformation {
		return {
			name: this.#state.persist ? "File System" : "Memory",
			properties: {
				...(this.#state.persist
					? { Data: this.#state.storagePath }
					: {}),
				Instances: this.#state.actorCountOnStartup.toString(),
			},
		};
	}

	extraStartupLog() {
		return {
			instances: this.#state.actorCountOnStartup,
			data: this.#state.storagePath,
		};
	}

	getOrCreateInspectorAccessToken() {
		return this.#state.getOrCreateInspectorAccessToken();
	}
}
