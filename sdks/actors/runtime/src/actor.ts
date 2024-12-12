import { Lock } from "@core/asyncutil/lock";
import type { ActorContext } from "@rivet-gg/actors-core";
import { assertExists, assertInstanceOf } from "@std/assert";
import { deadline } from "@std/async/deadline";
import { throttle } from "@std/async/unstable-throttle";
import type { Logger } from "@std/log/get-logger";
import { Hono, type Context as HonoContext } from "hono";
import { upgradeWebSocket } from "hono/deno";
import type { WSEvents } from "hono/ws";
import onChange from "on-change";
import { setupLogging } from "../../common/src/log.ts";
import { assertUnreachable } from "../../common/src/utils.ts";
import {
	type ProtocolFormat,
	ProtocolFormatSchema,
} from "../../protocol/src/ws/mod.ts";
import type * as wsToClient from "../../protocol/src/ws/to_client.ts";
import * as wsToServer from "../../protocol/src/ws/to_server.ts";
import { type ActorConfig, mergeActorConfig } from "./config.ts";
import {
	Connection,
	type IncomingWebSocketMessage,
	type OutgoingWebSocketMessage,
} from "./connection.ts";
import * as errors from "./errors.ts";
import { instanceLogger, logger } from "./log.ts";
import { Rpc } from "./rpc.ts";

const KEYS = {
	SCHEDULE: {
		SCHEDULE: ["actor", "schedule", "schedule"],
		EVENT_PREFIX: ["actor", "schedule", "event"],
		event(id: string): string[] {
			return [...this.EVENT_PREFIX, id];
		},
	},
	STATE: {
		INITIALIZED: ["actor", "state", "initialized"],
		DATA: ["actor", "state", "data"],
	},
};

// TODO: Instead of doing this, use a temp var for state and attempt to write
// it. Roll back state if fails to serialize.
function isJsonSerializable(value: unknown): boolean {
	// Handle primitive types directly
	if (value === null || value === undefined) return true;
	if (typeof value === "number") return Number.isFinite(value);
	if (typeof value === "boolean" || typeof value === "string") return true;

	// Handle arrays
	if (Array.isArray(value)) {
		return value.every(isJsonSerializable);
	}

	// Handle plain objects
	if (typeof value === "object") {
		// Reject if it's not a plain object
		if (Object.getPrototypeOf(value) !== Object.prototype) return false;

		// Check all values recursively
		return Object.values(value).every(isJsonSerializable);
	}

	return false;
}

export interface OnBeforeConnectOptions<ConnParams> {
	request: Request;
	parameters: ConnParams;
}

export interface SaveStateOptions {
	/**
	 * Forces the state to be saved immediately. This function will return when the state has save successfully.
	 */
	immediate?: boolean;
}

/** Actor type alias with all `any` types. Used for `extends` in classes referencing this actor. */
// biome-ignore lint/suspicious/noExplicitAny: Needs to be used in `extends`
export type AnyActor = Actor<any, any, any>;

export abstract class Actor<
	State = undefined,
	ConnParams = undefined,
	ConnState = undefined,
> {
	// Store the init promise so network requests can await initialization
	#initializedPromise?: Promise<void>;

	#stateChanged = false;

	/**
	 * The proxied state that notifies of changes automatically.
	 *
	 * If the object can't be proxied then this value will not be a proxy.
	 */
	#stateProxy!: State;

	/** Raw state without the proxy wrapper */
	#stateRaw!: State;

	#server?: Deno.HttpServer<Deno.NetAddr>;
	#backgroundPromises: Promise<void>[] = [];
	#config: ActorConfig;
	#ctx!: ActorContext;
	#ready = false;

	#connectionIdCounter = 0;
	#connections = new Set<Connection<this>>();
	#eventSubscriptions = new Map<string, Set<Connection<this>>>();

	protected constructor(config?: Partial<ActorConfig>) {
		this.#config = mergeActorConfig(config);

		this.#saveStateThrottled = throttle(() => {
			this.#saveStateInner().catch((error) => {
				logger().error("failed to save state", { error });
			});
		}, this.#config.state.saveInterval);
	}

	// This is called by Rivet when the actor is exported as the default
	// property
	public static start(ctx: ActorContext) {
		setupLogging();

		// biome-ignore lint/complexity/noThisInStatic lint/suspicious/noExplicitAny: Needs to construct self
		const instance = new (this as any)() as Actor;
		return instance.#run(ctx);
	}

	async #run(ctx: ActorContext) {
		this.#ctx = ctx;

		// Run server immediately since init might take a few ms
		this.#runServer();

		// Initialize server
		//
		// Store the promise so network requests can await initialization
		this.#initializedPromise = this.#initializeState();
		await this.#initializedPromise;
		this.#initializedPromise = undefined;

		// TODO: Exit process if this errors
		logger().info("starting");
		await this._onStart?.();

		logger().info("ready");
		this.#ready = true;
	}

	get #stateEnabled() {
		return typeof this._onInitialize === "function";
	}

	#validateStateEnabled() {
		if (!this.#stateEnabled) {
			throw new errors.StateNotEnabled();
		}
	}

	get #connectionStateEnabled() {
		return typeof this._onBeforeConnect === "function";
	}

	#saveStateLock = new Lock<void>(void 0);

	/** Throttled save state method. Used to write to KV at a reasonable cadence. */
	#saveStateThrottled: () => void;

	/** Promise used to wait for a save to complete. This is required since you cannot await `#saveStateThrottled`. */
	#onStateSavedPromise?: PromiseWithResolvers<void>;

	/** Saves the state to the database. You probably want to use #saveStateThrottled instead except for a few edge cases. */
	async #saveStateInner() {
		try {
			if (this.#stateChanged) {
				// Use a lock in order to avoid race conditions with multiple
				// parallel promises writing to KV. This should almost never happen
				// unless there are abnormally high latency in KV writes.
				await this.#saveStateLock.lock(async () => {
					logger().debug("saving state");

					// There might be more changes while we're writing, so we set this
					// before writing to KV in order to avoid a race condition.
					this.#stateChanged = false;

					// Write to KV
					await this.#ctx.kv.put(KEYS.STATE.DATA, this.#stateRaw);
				});
			}

			this.#onStateSavedPromise?.resolve();
		} catch (error) {
			this.#onStateSavedPromise?.reject(error);
			throw error;
		}
	}

	/** Updates the state and creates a new proxy. */
	#setStateWithoutChange(value: State) {
		if (!isJsonSerializable(value)) {
			throw new errors.InvalidStateType();
		}
		this.#stateProxy = this.#createStateProxy(value);
		this.#stateRaw = value;
	}

	#createStateProxy(target: State): State {
		// If this can't be proxied, return raw value
		if (target === null || typeof target !== "object") {
			if (!isJsonSerializable(target)) {
				throw new errors.InvalidStateType();
			}
			return target;
		}

		// Unsubscribe from old state
		if (this.#stateProxy) {
			onChange.unsubscribe(this.#stateProxy);
		}

		// Listen for changes to the object in order to automatically write state
		return onChange(
			target,
			// biome-ignore lint/suspicious/noExplicitAny: Don't know types in proxy
			(path: any, value: any, _previousValue: any, _applyData: any) => {
				if (!isJsonSerializable(value)) {
					throw new errors.InvalidStateType({ path });
				}
				this.#stateChanged = true;

				// Call onStateChange if it exists
				if (this._onStateChange && this.#ready) {
					try {
						this._onStateChange(this.#stateRaw);
					} catch (error) {
						logger().error("error in `_onStateChange`", { error });
					}
				}
			},
			{
				ignoreDetached: true,
			},
		);
	}

	async #initializeState() {
		if (!this.#stateEnabled) {
			logger().debug("state not enabled");
			return;
		}
		assertExists(this._onInitialize);

		// Read initial state
		const getStateBatch = await this.#ctx.kv.getBatch([
			KEYS.STATE.INITIALIZED,
			KEYS.STATE.DATA,
		]);
		const initialized = getStateBatch.get(KEYS.STATE.INITIALIZED) as boolean;
		const stateData = getStateBatch.get(KEYS.STATE.DATA) as State;

		if (!initialized) {
			// Initialize
			logger().info("initializing");
			const stateOrPromise = await this._onInitialize();

			let stateData: State;
			if (stateOrPromise instanceof Promise) {
				stateData = await stateOrPromise;
			} else {
				stateData = stateOrPromise;
			}

			// Update state
			logger().debug("writing state");
			await this.#ctx.kv.putBatch(
				new Map<unknown, unknown>([
					[KEYS.STATE.INITIALIZED, true],
					[KEYS.STATE.DATA, stateData],
				]),
			);
			this.#setStateWithoutChange(stateData);
		} else {
			// Save state
			logger().debug("already initialized");
			this.#setStateWithoutChange(stateData);
		}
	}

	#runServer() {
		const app = new Hono();

		app.get("/", (c) => {
			// TODO: Give the metadata about this actor (ie tags)
			return c.text("This is a Rivet Actor\n\nLearn more at https://rivet.gg");
		});

		//app.post("/rpc/:name", this.#pandleRpc.bind(this));

		app.get("/connect", upgradeWebSocket(this.#handleWebSocket.bind(this)));

		app.all("*", (c) => {
			return c.text("Not Found", 404);
		});

		const port = this.#getServerPort();
		logger().info("server running", { port });
		this.#server = Deno.serve({ port, hostname: "0.0.0.0" }, app.fetch);
	}

	#getServerPort(): number {
		const portStr = Deno.env.get("PORT_HTTP");
		if (!portStr) {
			throw "Missing port";
		}
		const port = Number.parseInt(portStr);
		if (!Number.isFinite(port)) {
			throw "Invalid port";
		}

		return port;
	}

	// MARK: RPC
	//async #handleRpc(c: HonoContext): Promise<Response> {
	//  // TODO: Wait for initialize
	//	try {
	//		const rpcName = c.req.param("name");
	//		const requestBody = await c.req.json<httpRpc.Request<unknown[]>>();
	//		const ctx = new RpcContext<ConnState>();
	//
	//		const output = await this.#executeRpc(ctx, rpcName, requestBody.args);
	//		return c.json({ output });
	//	} catch (error) {
	//		logger().error("RPC Error:", error);
	//		return c.json({ error: String(error) }, 500);
	//	} finally {
	//		await this.forceSaveState();
	//	}
	//}

	#isValidRpc(rpcName: string): boolean {
		// Prevent calling private methods
		if (rpcName.startsWith("#")) return false;

		// Prevent accidental leaking of private methods, since this is a common
		// convention
		if (rpcName.startsWith("_")) return false;

		// Prevent calling protected methods
		// TODO: Are there other RPC functions that should be private? i.e.	internal JS runtime functions? Should we validate the fn is part of this prototype?
		const reservedMethods = ["constructor", "initialize", "run"];
		if (reservedMethods.includes(rpcName)) return false;

		return true;
	}

	// MARK: Events
	#addSubscription(eventName: string, connection: Connection<this>) {
		connection.subscriptions.add(eventName);

		let subscribers = this.#eventSubscriptions.get(eventName);
		if (!subscribers) {
			subscribers = new Set();
			this.#eventSubscriptions.set(eventName, subscribers);
		}
		subscribers.add(connection);
	}

	#removeSubscription(eventName: string, connection: Connection<this>) {
		connection.subscriptions.delete(eventName);

		const subscribers = this.#eventSubscriptions.get(eventName);
		if (subscribers) {
			subscribers.delete(connection);
			if (subscribers.size === 0) {
				this.#eventSubscriptions.delete(eventName);
			}
		}
	}

	// MARK: WebSocket
	async #handleWebSocket(c: HonoContext): Promise<WSEvents<WebSocket>> {
		// Wait for init to finish
		if (this.#initializedPromise) await this.#initializedPromise;

		// TODO: Handle timeouts opening socket

		// Validate protocol
		const protocolVersion = c.req.query("version");
		if (protocolVersion !== "1") {
			logger().warn("invalid protocol version", { protocolVersion });
			throw new errors.InvalidProtocolVersion(protocolVersion);
		}

		const protocolFormatRaw = c.req.query("format");
		const { data: protocolFormat, success } =
			ProtocolFormatSchema.safeParse(protocolFormatRaw);
		if (!success) {
			logger().warn("invalid protocol format", {
				protocolFormat: protocolFormatRaw,
			});
			throw new errors.InvalidProtocolFormat(protocolFormatRaw);
		}

		// Validate params size (limiting to 4KB which is reasonable for query params)
		const paramsStr = c.req.query("params");
		if (
			paramsStr &&
			paramsStr.length > this.#config.protocol.maxConnectionParametersSize
		) {
			logger().warn("connection parameters too long");
			throw new errors.ConnectionParametersTooLong();
		}

		// Parse and validate params
		let params: ConnParams;
		try {
			params =
				typeof paramsStr === "string" ? JSON.parse(paramsStr) : undefined;
		} catch (error) {
			logger().warn("malformed connection parameters", { error });
			throw new errors.MalformedConnectionParameters(error);
		}

		// Authenticate connection
		let state: ConnState | undefined = undefined;
		const PREAPRE_CONNECT_TIMEOUT = 5000; // 5 seconds
		if (this._onBeforeConnect) {
			const dataOrPromise = this._onBeforeConnect({
				request: c.req.raw,
				parameters: params,
			});
			if (dataOrPromise instanceof Promise) {
				state = await deadline(dataOrPromise, PREAPRE_CONNECT_TIMEOUT);
			} else {
				state = dataOrPromise;
			}
		}

		let conn: Connection<this> | undefined;
		return {
			onOpen: (_evt, ws) => {
				// Create connection
				//
				// If `_onBeforeConnect` is not defined and `state` is
				// undefined, there will be a runtime error when attempting to
				// read it
				this.#connectionIdCounter += 1;
				// TODO: As any
				conn = new Connection<Actor<State, ConnParams, ConnState>>(
					this.#connectionIdCounter,
					ws,
					protocolFormat,
					state,
					this.#connectionStateEnabled,
				);
				this.#connections.add(conn);

				// Handle connection
				const CONNECT_TIMEOUT = 5000; // 5 seconds
				if (this._onConnect) {
					const voidOrPromise = this._onConnect(conn);
					if (voidOrPromise instanceof Promise) {
						deadline(voidOrPromise, CONNECT_TIMEOUT).catch((error) => {
							logger().error("error in `_onConnect`, closing socket", {
								error,
							});
							conn?.disconnect("`onConnect` failed");
						});
					}
				}
			},
			onMessage: async (evt) => {
				if (!conn) {
					logger().warn("`conn` does not exist");
					return;
				}

				let rpcRequestId: number | undefined;
				try {
					const value = evt.data.valueOf() as IncomingWebSocketMessage;

					// Validate value length
					let length: number;
					if (typeof value === "string") {
						length = value.length;
					} else if (value instanceof Blob) {
						length = value.size;
					} else if (
						value instanceof ArrayBuffer ||
						value instanceof SharedArrayBuffer
					) {
						length = value.byteLength;
					} else {
						assertUnreachable(value);
					}
					if (length > this.#config.protocol.maxIncomingMessageSize) {
						throw new errors.MessageTooLong();
					}

					// Parse & validate message
					const {
						data: message,
						success,
						error,
					} = wsToServer.ToServerSchema.safeParse(await conn._parse(value));
					if (!success) {
						throw new errors.MalformedMessage(error);
					}

					if ("rr" in message.body) {
						// RPC request

						const { i: id, n: name, a: args = [] } = message.body.rr;

						rpcRequestId = id;

						const ctx = new Rpc<this>(conn);
						const output = await this.#executeRpc(ctx, name, args);

						conn._sendWebSocketMessage(
							conn._serialize({
								body: {
									ro: {
										i: id,
										o: output,
									},
								},
							} satisfies wsToClient.ToClient),
						);
					} else if ("sr" in message.body) {
						// Subscription request

						const { e: eventName, s: subscribe } = message.body.sr;

						if (subscribe) {
							this.#addSubscription(eventName, conn);
						} else {
							this.#removeSubscription(eventName, conn);
						}
					} else {
						assertUnreachable(message.body);
					}
				} catch (error) {
					// Build response error information. Only return errors if flagged as public in order to prevent leaking internal behavior.
					let code: string;
					let message: string;
					let metadata: unknown = undefined;
					if (error instanceof errors.ActorError && error.public) {
						logger().info("connection public error", {
							rpc: rpcRequestId,
							error,
						});

						code = error.code;
						message = String(error);
						metadata = error.metadata;
					} else {
						logger().warn("connection internal error", {
							rpc: rpcRequestId,
							error,
						});

						code = errors.INTERNAL_ERROR_CODE;
						message = errors.INTERNAL_ERROR_DESCRIPTION;
					}

					// Build response
					if (rpcRequestId !== undefined) {
						conn._sendWebSocketMessage(
							conn._serialize({
								body: {
									re: {
										i: rpcRequestId,
										c: code,
										m: message,
										md: metadata,
									},
								},
							} satisfies wsToClient.ToClient),
						);
					} else {
						conn._sendWebSocketMessage(
							conn._serialize({
								body: {
									er: {
										c: code,
										m: message,
										md: metadata,
									},
								},
							} satisfies wsToClient.ToClient),
						);
					}
				}
			},
			onClose: () => {
				if (!conn) {
					logger().warn("`conn` does not exist");
					return;
				}

				this.#connections.delete(conn);

				// Remove subscriptions
				for (const eventName of [...conn.subscriptions.values()]) {
					this.#removeSubscription(eventName, conn);
				}

				this._onDisconnect?.(conn);
			},
			onError: (error) => {
				// Actors don't need to know about this, since it's abstracted
				// away
				logger().warn("websocket error", { error });
			},
		};
	}

	#assertReady() {
		if (!this.#ready) throw new errors.InternalError("Actor not ready");
	}

	async #executeRpc(
		ctx: Rpc<this>,
		rpcName: string,
		args: unknown[],
	): Promise<unknown> {
		// Prevent calling private or reserved methods
		if (!this.#isValidRpc(rpcName)) {
			logger().warn("attempted to call invalid rpc", { rpcName });
			throw new errors.RpcNotFound();
		}

		// Check if the method exists on this object
		// biome-ignore lint/suspicious/noExplicitAny: RPC name is dynamic from client
		const rpcFunction = (this as any)[rpcName];
		if (typeof rpcFunction !== "function") {
			logger().warn("rpc not found", { rpcName });
			throw new errors.RpcNotFound();
		}

		// TODO: pass abortable to the rpc to decide when to abort
		// TODO: Manually call abortable for better error handling
		// Call the function on this object with those arguments
		try {
			const outputOrPromise = rpcFunction.call(this, ctx, ...args);
			if (outputOrPromise instanceof Promise) {
				return await deadline(outputOrPromise, this.#config.rpc.timeout);
			} else {
				return outputOrPromise;
			}
		} catch (error) {
			if (error instanceof DOMException && error.name === "TimeoutError") {
				throw new errors.RpcTimedOut();
			} else {
				throw error;
			}
		} finally {
			this.#saveStateThrottled();
		}
	}

	// MARK: Lifecycle hooks
	protected _onInitialize?(): State | Promise<State>;

	protected _onStart?(): void | Promise<void>;

	protected _onStateChange?(newState: State): void | Promise<void>;

	protected _onBeforeConnect?(
		opts: OnBeforeConnectOptions<ConnParams>,
	): ConnState | Promise<ConnState>;

	protected _onConnect?(connection: Connection<this>): void | Promise<void>;

	protected _onDisconnect?(connection: Connection<this>): void | Promise<void>;

	// MARK: Exposed methods
	protected get _log(): Logger {
		return instanceLogger();
	}

	protected get _connections(): Set<Connection<this>> {
		return this.#connections;
	}

	protected get _state(): State {
		this.#validateStateEnabled();
		return this.#stateProxy;
	}

	protected set _state(value: State) {
		this.#validateStateEnabled();
		this.#setStateWithoutChange(value);
		this.#stateChanged = true;
	}

	/**
	 * Broadcasts an event to all connected clients.
	 */
	protected _broadcast<Args extends Array<unknown>>(
		name: string,
		...args: Args
	) {
		this.#assertReady();

		// Send to all connected clients
		const subscriptions = this.#eventSubscriptions.get(name);
		if (!subscriptions) return;

		const toClient: wsToClient.ToClient = {
			body: {
				ev: {
					n: name,
					a: args,
				},
			},
		};

		// Send message to clients
		const serialized: Record<string, OutgoingWebSocketMessage> = {};
		for (const connection of subscriptions) {
			// Lazily serialize the appropriate format
			if (!(connection._protocolFormat in serialized)) {
				serialized[connection._protocolFormat] =
					connection._serialize(toClient);
			}

			connection._sendWebSocketMessage(serialized[connection._protocolFormat]);
		}
	}

	/**
	 * Runs a promise in the background.
	 *
	 * This allows the actor runtime to ensure that a promise completes while
	 * returning from an RPC request early.
	 */
	protected _runInBackground(promise: Promise<void>) {
		this.#assertReady();

		// TODO: Should we force save the state?
		// Add logging to promise and make it non-failable
		const nonfailablePromise = promise
			.then(() => {
				logger().debug("background promise complete");
			})
			.catch((error) => {
				logger().error("background promise failed", { error });
			});
		this.#backgroundPromises.push(nonfailablePromise);
	}

	/**
	 * Forces the state to get saved.
	 *
	 * This is helpful if running a long task that may fail later or when
	 * running a background job that updates the state.
	 */
	protected async _saveState(opts: SaveStateOptions) {
		this.#assertReady();

		if (this.#stateChanged) {
			if (opts.immediate) {
				// Save immediately
				await this.#saveStateInner();
			} else {
				// Create callback
				if (!this.#onStateSavedPromise) {
					this.#onStateSavedPromise = Promise.withResolvers();
				}

				// Save state throttled
				this.#saveStateThrottled();

				// Wait for save
				await this.#onStateSavedPromise.promise;
			}
		}
	}

	protected async _shutdown() {
		// Stop accepting new connections
		if (this.#server) await this.#server.shutdown();

		// Disconnect existing connections
		const promises: Promise<unknown>[] = [];
		for (const connection of this.#connections) {
			const raw = connection._websocket.raw;
			if (!raw) continue;

			// Create deferred promise
			const { promise, resolve } = Promise.withResolvers();
			assertExists(resolve, "resolve should be defined by now");

			// Resolve promise when websocket closes
			raw.addEventListener("close", resolve);

			// Close connection
			connection.disconnect();

			promises.push(promise);
		}

		// Await all `close` event listeners with 1.5 second timeout
		const res = Promise.race([
			Promise.all(promises).then(() => false),
			new Promise<boolean>((res) => setTimeout(() => res(true), 1500)),
		]);

		if (await res) {
			logger().warn(
				"timed out waiting for connections to close, shutting down anyway",
			);
		}

		Deno.exit(0);
	}
}
