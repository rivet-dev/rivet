import type { Context as HonoContext } from "hono";
import invariant from "invariant";
import { ActorStopping } from "@/actor/errors";
import { type ActorRouter, createActorRouter } from "@/actor/router";
import { routeWebSocket } from "@/actor/router-websocket-endpoints";
import { createClientWithDriver } from "@/client/client";
import { ClientConfigSchema } from "@/client/config";
import { createInlineWebSocket } from "@/common/inline-websocket-adapter";
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
import type { ManagerDisplayInformation } from "@/manager/driver";
import type { Encoding, UniversalWebSocket } from "@/mod";
import type { DriverConfig, RegistryConfig } from "@/registry/config";
import type * as schema from "@/schemas/file-system-driver/mod";
import type { GetUpgradeWebSocket } from "@/utils";
import type { FileSystemGlobalState } from "./global-state";
import { logger } from "./log";
import { generateActorId } from "./utils";

export class FileSystemManagerDriver implements ManagerDriver {
	#config: RegistryConfig;
	#state: FileSystemGlobalState;
	#driverConfig: DriverConfig;
	#getUpgradeWebSocket: GetUpgradeWebSocket | undefined;

	#actorDriver: ActorDriver;
	#actorRouter: ActorRouter;

	constructor(
		config: RegistryConfig,
		state: FileSystemGlobalState,
		driverConfig: DriverConfig,
	) {
		this.#config = config;
		this.#state = state;
		this.#driverConfig = driverConfig;

		// Actors run on the same node as the manager, so we create a dummy actor router that we route requests to
		const inlineClient = createClientWithDriver(this);

		this.#actorDriver = this.#driverConfig.actor(
			config,
			this,
			inlineClient,
		);
		this.#actorRouter = createActorRouter(
			this.#config,
			this.#actorDriver,
			undefined,
			config.test.enabled,
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
		// Normalize the path (add leading slash if needed) but preserve query params
		const normalizedPath = path.startsWith("/") ? path : `/${path}`;

		// Create a fake request with the full URL including query parameters
		const fakeUrl = `http://inline-actor${normalizedPath}`;
		const fakeRequest = new Request(fakeUrl, {
			method: "GET",
		});

		// Extract just the pathname for routing (without query params)
		const pathOnly = normalizedPath.split("?")[0];
		const { gatewayId, requestId } = createHibernatableRequestMetadata();

		const wsHandler = await routeWebSocket(
			fakeRequest,
			pathOnly,
			{},
			this.#config,
			this.#actorDriver,
			actorId,
			encoding,
			params,
			gatewayId,
			requestId,
			true,
			false,
		);
		return createInlineWebSocket(wsHandler);
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
		const upgradeWebSocket = this.#getUpgradeWebSocket?.();
		invariant(upgradeWebSocket, "missing getUpgradeWebSocket");

		// Handle raw WebSocket paths
		const pathOnly = path.split("?")[0];
		const normalizedPath = pathOnly.startsWith("/")
			? pathOnly
			: `/${pathOnly}`;
		const { gatewayId, requestId } = createHibernatableRequestMetadata();
		const wsHandler = await routeWebSocket(
			// TODO: Create new request with new path
			c.req.raw,
			normalizedPath,
			c.req.header(),
			this.#config,
			this.#actorDriver,
			actorId,
			encoding,
			params,
			gatewayId,
			requestId,
			true,
			false,
		);
		return upgradeWebSocket(() => wsHandler)(c, noopNext());
	}

	async buildGatewayUrl(actorId: string): Promise<string> {
		const port = this.#config.managerPort ?? 6420;
		return `http://127.0.0.1:${port}/gateway/${encodeURIComponent(actorId)}`;
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

		return actorStateToOutput(actor.state);
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
			return actorStateToOutput(actor.state);
		}

		return undefined;
	}

	async getOrCreateWithKey(
		input: GetOrCreateWithKeyInput,
	): Promise<ActorOutput> {
		// Generate the deterministic actor ID
		const actorId = generateActorId(input.name, input.key);

		// Use the atomic getOrCreateActor method
		await this.#state.loadOrCreateActor(
			actorId,
			input.name,
			input.key,
			input.input,
		);

		// Start the actor immediately so timestamps are set
		await this.#actorDriver.loadActor(actorId);

		// Reload state to get updated timestamps
		const state = await this.#state.loadActorStateOrError(actorId);
		return actorStateToOutput(state);
	}

	async createActor({ name, key, input }: CreateInput): Promise<ActorOutput> {
		// Generate the deterministic actor ID
		const actorId = generateActorId(name, key);

		await this.#state.createActor(actorId, name, key, input);

		// Start the actor immediately so timestamps are set
		await this.#actorDriver.loadActor(actorId);

		// Reload state to get updated timestamps
		const state = await this.#state.loadActorStateOrError(actorId);
		return actorStateToOutput(state);
	}

	async listActors({ name }: ListActorsInput): Promise<ActorOutput[]> {
		const actors: ActorOutput[] = [];
		const itr = this.#state.getActorsIterator({});

		for await (const actor of itr) {
			if (actor.name === name) {
				actors.push(actorStateToOutput(actor));
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

	async kvGet(actorId: string, key: Uint8Array): Promise<string | null> {
		const response = await this.#state.kvBatchGet(actorId, [key]);
		return response[0] !== null
			? new TextDecoder().decode(response[0])
			: null;
	}

	displayInformation(): ManagerDisplayInformation {
		return {
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

	setGetUpgradeWebSocket(getUpgradeWebSocket: GetUpgradeWebSocket): void {
		this.#getUpgradeWebSocket = getUpgradeWebSocket;
	}
}

function actorStateToOutput(state: schema.ActorState): ActorOutput {
	return {
		actorId: state.actorId,
		name: state.name,
		key: state.key as string[],
		createTs: Number(state.createdAt),
		startTs: state.startTs !== null ? Number(state.startTs) : null,
		connectableTs:
			state.connectableTs !== null ? Number(state.connectableTs) : null,
		sleepTs: state.sleepTs !== null ? Number(state.sleepTs) : null,
		destroyTs: state.destroyTs !== null ? Number(state.destroyTs) : null,
	};
}

function createHibernatableRequestMetadata(): {
	gatewayId: ArrayBuffer;
	requestId: ArrayBuffer;
} {
	const gatewayId = new Uint8Array(4);
	const requestId = new Uint8Array(4);
	crypto.getRandomValues(gatewayId);
	crypto.getRandomValues(requestId);
	return {
		gatewayId: gatewayId.buffer.slice(0),
		requestId: requestId.buffer.slice(0),
	};
}
