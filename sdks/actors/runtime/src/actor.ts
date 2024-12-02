import Rivet from "@rivet-gg/actors-core";
import { deadline } from "@std/async/deadline";
import { Context as HonoContext, Hono } from "hono";
import { WSEvents } from "hono/ws";
import { upgradeWebSocket } from "hono/deno";
import { Lock } from "@core/asyncutil/lock";
import { ActorConfig, mergeActorConfig } from "./config.ts";
import onChange from "on-change";
import { Connection } from "./connection.ts";
import { RpcContext } from "./rpc_context.ts";
import * as httpRpc from "../../protocol/src/http/rpc.ts";
import * as wsToClient from "../../protocol/src/ws/to_client.ts";
import * as wsToServer from "../../protocol/src/ws/to_server.ts";
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

export abstract class Actor<State = undefined, ConnectionData = undefined> {
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

	#backgroundPromises: Promise<void>[] = [];
	#config: ActorConfig;
	#ready: boolean = false;

	#connections = new Set<Connection<ConnectionData>>();
	#eventSubscriptions = new Map<string, Set<Connection<ConnectionData>>>();

	get connections(): Set<Connection<ConnectionData>> {
		return this.#connections;
	}

	public constructor(config?: Partial<ActorConfig>) {
		this.#config = mergeActorConfig(config);
	}

	protected initializeState?(): State | Promise<State>;

	protected onStart?(): void | Promise<void>;

	protected onConnect?(
		connection: Connection<ConnectionData>,
		...args: unknown[]
	): ConnectionData | Promise<ConnectionData>;

	protected onDisconnect?(
		connection: Connection<ConnectionData>,
	): void | Promise<void>;

	protected onStateChange?(newState: State): void | Promise<void>;

	public async run() {
		await this.#initializeState();

		this.#runServer();

		// TODO: Exit process if this errors
		console.log("calling start");
		await this.onStart?.();

		console.log("ready");
		this.#ready = true;
	}

	public get state(): State {
		this.#validateStateEnabled();
		return this.#stateProxy;
	}

	public set state(value: State) {
		this.#validateStateEnabled();
		this.#setStateWithoutChange(value);
		this.#stateChanged = true;
	}

	get #stateEnabled() {
		return typeof this.initializeState == "function";
	}

	#validateStateEnabled() {
		if (!this.#stateEnabled) {
			throw new Error(
				"State not enabled. Must implement initializeState to use state.",
			);
		}
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
					throw new Error(
						`State value at path "${path}" must be JSON serializable`,
					);
				}
				this.#stateChanged = true;

				// Call onStateChange if it exists
				if (this.onStateChange && this.#ready) {
					try {
						this.onStateChange(this.#stateRaw);
					} catch (err) {
						console.error("Error from onStateChange:", err);
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
			console.log("State not enabled");
			return;
		}
		assertExists(this.initializeState);

		// Read initial state
		const getStateBatch = await Rivet.kv.getBatch([
			KEYS.STATE.INITIALIZED,
			KEYS.STATE.DATA,
		]);
		const initialized = getStateBatch.get(KEYS.STATE.INITIALIZED);
		const stateData = getStateBatch.get(KEYS.STATE.DATA);

		if (!initialized) {
			// Initialize
			console.log("initializing");
			let stateOrPromise = await this.initializeState();

			let stateData: State;
			if (stateOrPromise instanceof Promise) {
				stateData = await stateOrPromise;
			} else {
				stateData = stateOrPromise;
			}

			// Update state
			console.log("writing initial state");
			await Rivet.kv.putBatch(
				new Map<unknown, unknown>([
					[KEYS.STATE.INITIALIZED, true],
					[KEYS.STATE.DATA, stateData],
				]),
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
			return c.text(
				"This is a Rivet Actor\n\nLearn more at https://rivet.gg",
			);
		});

		app.post("/rpc/:name", this.#handleRpc.bind(this));

		app.get(
			"/connect",
			upgradeWebSocket(this.#handleWebSocket.bind(this)),
		);

		app.all("*", (c) => {
			return c.text("Not Found", 404);
		});

		const port = this.#getServerPort();
		console.log(`server running on ${port}`);
		Deno.serve({ port, hostname: "0.0.0.0" }, app.fetch);
	}

	#getServerPort(): number {
		const portStr = Deno.env.get("PORT_http");
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
	async #handleRpc(c: HonoContext): Promise<Response> {
		try {
			const rpcName = c.req.param("name");
			const requestBody = await c.req.json<httpRpc.Request<unknown[]>>();
			const ctx = new RpcContext<ConnectionData>();

			const output = await this.#executeRpc(ctx, rpcName, requestBody.args);
			return c.json({ output });
		} catch (error) {
			console.error("RPC Error:", error);
			return c.json({ error: String(error) }, 500);
		} finally {
			await this.forceSaveState();
		}
	}

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
	broadcast<Args extends Array<unknown>>(name: string, ...args: Args) {
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
			connection.sendWebSocketMessage(body);
		}
	}

	#addSubscription(eventName: string, connection: Connection<ConnectionData>) {
		connection.subscriptions.add(eventName);

        let subscribers = this.#eventSubscriptions.get(eventName);
        if (!subscribers) {
            subscribers = new Set();
            this.#eventSubscriptions.set(eventName, subscribers);
        }
        subscribers.add(connection);
    }

    #removeSubscription(eventName: string, connection: Connection<ConnectionData>) {
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
		const conn = new Connection<ConnectionData>();

		// Add connection to set
		this.#connections.add(conn);

		// TODO: Handle timeouts opening socket

		// Validate args size (limiting to 4KB which is reasonable for query params)
		const MAX_ARGS_SIZE = 4096;
		const argsStr = c.req.query("args");
		if (argsStr && argsStr.length > MAX_ARGS_SIZE) {
			throw new Error(`WebSocket args too large (max ${MAX_ARGS_SIZE} bytes)`);
		}

		// Parse and validate args
		let args: unknown[];
		try {
			args = typeof argsStr === "string" ? JSON.parse(argsStr) : [];
			if (!Array.isArray(args)) {
				throw new Error("WebSocket args must be an array");
			}
		} catch (err) {
			throw new Error(`Invalid WebSocket args: ${err}`);
		}

		// Handle connection with timeout
		const CONNECT_TIMEOUT = 5000; // 5 seconds
		if (this.onConnect) {
			const dataOrPromise = this.onConnect(conn, ...args);
			let data: ConnectionData;
			if (dataOrPromise instanceof Promise) {
				data = await deadline(dataOrPromise, CONNECT_TIMEOUT);
			} else {
				data = dataOrPromise;
			}
			conn.data = data;
		}

		return {
			onOpen: (_evt, ws) => {
				// Actors don't need to know about this. Events are
				// automatically queued.
				conn.websocket = ws;
			},
			onMessage: async (evt, ws) => {
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

						const ctx = new RpcContext<ConnectionData>();
						const output = await this.#executeRpc(ctx, name, args);

						ws.send(JSON.stringify(
							{
								body: {
									rpcResponseOk: {
										id,
										output,
									},
								},
							} satisfies wsToClient.ToClient,
						));
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
						ws.send(JSON.stringify(
							{
								body: {
									rpcResponseError: {
										id: rpcRequestId,
										message: String(err),
									},
								},
							} satisfies wsToClient.ToClient,
						));
					} else {
						ws.send(JSON.stringify(
							{
								body: {
									error: {
										message: String(err),
									},
								},
							} satisfies wsToClient.ToClient,
						));
					}
				}
			},
			onClose: () => {
				this.#connections.delete(conn);

				// Remove subscriptions
				for (const eventName of [...conn.subscriptions.values()]) {
					this.#removeSubscription(eventName, conn);
				}

				this.onDisconnect?.(conn);
			},
			onError: (evt) => {
				// Actors don't need to know about this, since it's abstracted
				// away
				console.warn("WebSocket error:", evt);
			},
		};
	}

	/**
	 * Runs a promise in the background.
	 *
	 * This allows the actor runtime to ensure that a promise completes while
	 * returning from an RPC request early.
	 */
	protected runInBackground(
		promise: Promise<void>,
	) {
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
	public async forceSaveState() {
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
				await Rivet.kv.put(KEYS.STATE.DATA, this.#stateRaw);
			} else {
				console.log("skipping save, state not modified");
			}
		});
	}

	#assertReady() {
		if (!this.#ready) throw new Error("Actor not ready");
	}

	async #executeRpc(
		ctx: RpcContext<ConnectionData>,
		rpcName: string,
		args: unknown[],
	): Promise<unknown> {
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
				return await deadline(
					outputOrPromise,
					this.#config.rpc.timeout,
				);
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
}
