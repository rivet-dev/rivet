import { ActorContext } from "@rivet-gg/actors-core";
import { deadline } from "@std/async/deadline";
import { Context as HonoContext, Hono } from "hono";
import { WSEvents } from "hono/ws";
import { upgradeWebSocket } from "hono/deno";
import { Lock } from "@core/asyncutil/lock";
import { ActorConfig, mergeActorConfig } from "./config.ts";
import onChange from "on-change";
import { Connection } from "./connection.ts";
import { Rpc } from "./rpc.ts";
import * as wsToClient from "../../protocol/src/ws/to_client.ts";
import * as wsToServer from "../../protocol/src/ws/to_server.ts";
import { MAX_CONN_PARAMS_SIZE } from "../../common/src/network.ts";
import { assertUnreachable } from "../../common/src/utils.ts";
import { assertExists } from "@std/assert";

const KEYS = {
	SCHEDULE: {
		SCHEDULE: ["actor", "schedule", "schedule"],
		EVENT_PREFIX: ["actor", "schedule", "event"],
		event(id: string): string[] {
			return [...this.EVENT_PREFIX, id];
		},
	},
	// Shutting down is not part of the state because you can't meaningfully handle the state change
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

export interface OnBeforeConnectOpts<ConnParams> {
	request: Request;
	parameters: ConnParams;
}

export abstract class Actor<
	State = undefined,
	ConnParams = undefined,
	ConnState = undefined,
> {
	// Store the init promise so network requests can await initialization
	#initializedPromise?: Promise<void>;

	#stateChanged: boolean = false;

	/**
	 * The proxied state that notifies of changes automatically.
	 *
	 * If the object can't be proxied then this value will not be a proxy.
	 */
	#stateProxy!: State;

	/** Raw state without the proxy wrapper */
	#stateRaw!: State;

	#saveStateLock = new Lock<void>(void 0);

	#server?: Deno.HttpServer<Deno.NetAddr>;
	#backgroundPromises: Promise<void>[] = [];
	#config: ActorConfig;
	#ctx!: ActorContext;
	#ready: boolean = false;

	#connectionIdCounter: number = 0;
	#connections = new Set<Connection<this>>();
	#eventSubscriptions = new Map<string, Set<Connection<this>>>();

	protected constructor(config?: Partial<ActorConfig>) {
		this.#config = mergeActorConfig(config);
	}

	protected get connections(): Set<Connection<this>> {
		return this.#connections;
	}

	protected _onInitialize?(): State | Promise<State>;

	protected _onStart?(): void | Promise<void>;

	protected _onStateChange?(newState: State): void | Promise<void>;

	protected _onBeforeConnect?(opts: OnBeforeConnectOpts<ConnParams>): ConnState | Promise<ConnState>;

	protected _onConnect?(connection: Connection<this>): void | Promise<void>;

	protected _onDisconnect?(connection: Connection<this>): void | Promise<void>;

	// This is called by Rivet when the actor is exported as the default
	// property
	public static start(ctx: ActorContext) {
		// deno-lint-ignore no-explicit-any
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
		console.log("calling start");
		await this._onStart?.();

		console.log("ready");
		this.#ready = true;
	}

	protected get state(): State {
		this.#validateStateEnabled();
		return this.#stateProxy;
	}

	protected set state(value: State) {
		this.#validateStateEnabled();
		this.#setStateWithoutChange(value);
		this.#stateChanged = true;
	}

	get #stateEnabled() {
		return typeof this._onInitialize == "function";
	}

	#validateStateEnabled() {
		if (!this.#stateEnabled) {
			throw new Error("State not enabled. Must implement createState to use state.");
		}
	}

	get #connectionStateEnabled() {
		return typeof this._onBeforeConnect == "function";
	}

	/** Updates the state and creates a new proxy. */
	#setStateWithoutChange(value: State) {
		if (!isJsonSerializable(value)) {
			throw new Error("State must be JSON serializable");
		}
		this.#stateProxy = this.#createStateProxy(value);
		this.#stateRaw = value;
	}

	#createStateProxy(target: State): State {
		// If this can't be proxied, return raw value
		if (target === null || typeof target !== "object") {
			if (!isJsonSerializable(target)) {
				throw new Error("State value must be JSON serializable");
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
			// deno-lint-ignore no-explicit-any
			(path: any, value: any, _previousValue: any, _applyData: any) => {
				if (!isJsonSerializable(value)) {
					throw new Error(`State value at path "${path}" must be JSON serializable`);
				}
				this.#stateChanged = true;

				// Call onStateChange if it exists
				if (this._onStateChange && this.#ready) {
					try {
						this._onStateChange(this.#stateRaw);
					} catch (err) {
						console.error("Error from onStateChange:", err);
					}
				}
			},
			{
				ignoreDetached: true,
			}
		);
	}

	async #initializeState() {
		if (!this.#stateEnabled) {
			console.log("State not enabled");
			return;
		}
		assertExists(this._onInitialize);

		// Read initial state
		const getStateBatch = await this.#ctx.kv.getBatch([KEYS.STATE.INITIALIZED, KEYS.STATE.DATA]);
		const initialized = getStateBatch.get(KEYS.STATE.INITIALIZED);
		const stateData = getStateBatch.get(KEYS.STATE.DATA);

		if (!initialized) {
			// Initialize
			console.log("initializing");
			const stateOrPromise = await this._onInitialize();

			let stateData: State;
			if (stateOrPromise instanceof Promise) {
				stateData = await stateOrPromise;
			} else {
				stateData = stateOrPromise;
			}

			// Update state
			console.log("writing initial state");
			await this.#ctx.kv.putBatch(
				new Map<unknown, unknown>([
					[KEYS.STATE.INITIALIZED, true],
					[KEYS.STATE.DATA, stateData],
				])
			);
			this.#setStateWithoutChange(stateData);
		} else {
			// Save state
			console.log("found existing state");
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
		console.log(`server running on ${port}`);
		this.#server = Deno.serve({ port, hostname: "0.0.0.0" }, app.fetch);
	}

	#getServerPort(): number {
		const portStr = Deno.env.get("PORT_HTTP");
		if (!portStr) {
			throw "Missing port";
		}
		const port = parseInt(portStr);
		if (!isFinite(port)) {
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
	//		console.error("RPC Error:", error);
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
		if (protocolVersion != "1") {
			throw new Error(`Invalid protocol version: ${protocolVersion}`);
		}

		// Validate params size (limiting to 4KB which is reasonable for query params)
		const paramsStr = c.req.query("params");
		if (paramsStr && paramsStr.length > MAX_CONN_PARAMS_SIZE) {
			throw new Error(`WebSocket params too large (max ${MAX_CONN_PARAMS_SIZE} bytes)`);
		}

		// Parse and validate params
		let params: ConnParams;
		try {
			params = typeof paramsStr === "string" ? JSON.parse(paramsStr) : undefined;
		} catch (err) {
			throw new Error(`Invalid WebSocket params: ${err}`);
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
					state!,
					this.#connectionStateEnabled
				);
				this.#connections.add(conn);

				// Handle connection
				const CONNECT_TIMEOUT = 5000; // 5 seconds
				if (this._onConnect) {
					const voidOrPromise = this._onConnect(conn);
					if (voidOrPromise instanceof Promise) {
						deadline(voidOrPromise, CONNECT_TIMEOUT).catch((err) => {
							console.error("Error in `onConnect`, closing socket:", err);
							conn?.disconnect("`onConnect` failed");
						});
					}
				}
			},
			onMessage: async (evt, ws) => {
				if (!conn) {
					console.warn("`conn` does not exist");
					return;
				}

				let rpcRequestId: string | undefined;
				try {
					const value = evt.data.valueOf();
					if (typeof value != "string") {
						throw new Error("message must be string");
					}
					// TODO: Validate message
					const message: wsToServer.ToServer = JSON.parse(value);

					if ("rpcRequest" in message.body) {
						const { id, name, args = [] } = message.body.rpcRequest;
						rpcRequestId = id;

						const ctx = new Rpc<this>(conn);
						const output = await this.#executeRpc(ctx, name, args);

						ws.send(
							JSON.stringify({
								body: {
									rpcResponseOk: {
										id,
										output,
									},
								},
							} satisfies wsToClient.ToClient)
						);
					} else if ("subscriptionRequest" in message.body) {
						if (message.body.subscriptionRequest.subscribe) {
							this.#addSubscription(message.body.subscriptionRequest.eventName, conn);
						} else {
							this.#removeSubscription(message.body.subscriptionRequest.eventName, conn);
						}
					} else {
						assertUnreachable(message.body);
					}
				} catch (err) {
					if (rpcRequestId) {
						ws.send(
							JSON.stringify({
								body: {
									rpcResponseError: {
										id: rpcRequestId,
										message: String(err),
									},
								},
							} satisfies wsToClient.ToClient)
						);
					} else {
						ws.send(
							JSON.stringify({
								body: {
									error: {
										message: String(err),
									},
								},
							} satisfies wsToClient.ToClient)
						);
					}
				}
			},
			onClose: () => {
				if (!conn) {
					console.warn!("`conn` does not exist");
					return;
				}

				this.#connections.delete(conn);

				// Remove subscriptions
				for (const eventName of [...conn.subscriptions.values()]) {
					this.#removeSubscription(eventName, conn);
				}

				this._onDisconnect?.(conn);
			},
			onError: (evt) => {
				// Actors don't need to know about this, since it's abstracted
				// away
				console.warn("WebSocket error:", evt);
			},
		};
	}

	#assertReady() {
		if (!this.#ready) throw new Error("Actor not ready");
	}

	async #executeRpc(ctx: Rpc<this>, rpcName: string, args: unknown[]): Promise<unknown> {
		// Prevent calling private or reserved methods
		if (!this.#isValidRpc(rpcName)) {
			throw new Error(`RPC ${rpcName} is not accessible`);
		}

		// Check if the method exists on this object
		// deno-lint-ignore no-explicit-any
		const rpcFunction = (this as any)[rpcName];
		if (typeof rpcFunction !== "function") {
			throw new Error(`RPC ${rpcName} not found`);
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
			if (error instanceof DOMException && error.name == "TimeoutError") {
				throw new Error(`RPC ${rpcName} timed out`);
			} else {
				throw new Error(`RPC ${rpcName} failed: ${String(error)}`);
			}
		}
	}

	// MARK: Exposed methods
	/**
	 * Broadcasts an event to all connected clients.
	 */
	protected _broadcast<Args extends Array<unknown>>(name: string, ...args: Args) {
		this.#assertReady();

		// Send to all connected clients
		const subscriptions = this.#eventSubscriptions.get(name);
		if (!subscriptions) return;

		const body = JSON.stringify({
			body: {
				event: {
					name,
					args,
				},
			},
		} satisfies wsToClient.ToClient);
		for (const connection of subscriptions) {
			connection._sendWebSocketMessage(body);
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
			.then(() => console.log("background promise complete"))
			.catch((err) => {
				console.error("background promise failed", err);
				// ctx.log.error(
				// 	"background promise failed",
				// 	...errorToLogEntries("error", err),
				// )
			});
		this.#backgroundPromises.push(nonfailablePromise);
	}

	/**
	 * Forces the state to get saved.
	 *
	 * This is helpful if running a long task that may fail later or a background
	 * job that updates the state.
	 */
	protected async _forceSaveState() {
		this.#assertReady();

		// Use a lock in order to avoid race conditions with writing to KV
		await this.#saveStateLock.lock(async () => {
			if (this.#stateChanged) {
				console.log("saving state");

				// There might be more changes while we're writing, so we set
				// this before writing to KV in order to avoid a race
				// condition.
				this.#stateChanged = false;

				// Write to KV
				await this.#ctx.kv.put(KEYS.STATE.DATA, this.#stateRaw);
			} else {
				console.log("skipping save, state not modified");
			}
		});
	}

	protected async _shutdown() {
		// Stop accepting new connections
		if (this.#server) await this.#server.shutdown();

		// Disconnect existing connections
		let promises: Promise<unknown>[] = [];
		for (let connection of this.#connections) {
			let raw = connection._websocket.raw;
			if (!raw) continue;

			// Create deferred promise
			let resolve: (value: unknown) => void;
			let promise = new Promise((res) => (resolve = res));

			// Resolve promise when websocket closes
			raw.addEventListener("close", resolve!);

			// Close connection
			connection.disconnect();

			promises.push(promise);
		}

		// Await all `close` event listeners with 1.5 second timeout
		let res = Promise.race([
			Promise.all(promises).then(() => false),
			new Promise<boolean>((res) => setTimeout(() => res(true), 1500)),
		]);

		if (await res) {
			console.warn("timed out waiting for connections to close, shutting down anyway");
		}

		Deno.exit(0);
	}
}
