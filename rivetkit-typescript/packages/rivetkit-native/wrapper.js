/**
 * Thin JS wrapper that adapts native callback envelopes to the
 * EnvoyConfig callback shape used by the TypeScript envoy client.
 *
 * The native addon sends JSON envelopes with a "kind" field.
 * This wrapper routes them to the appropriate EnvoyConfig callbacks.
 */

const native = require("./index");

// CloseEvent was added to Node.js in v22. Polyfill for older versions.
if (typeof CloseEvent === "undefined") {
	global.CloseEvent = class CloseEvent extends Event {
		constructor(type, init = {}) {
			super(type);
			this.code = init.code ?? 0;
			this.reason = init.reason ?? "";
			this.wasClean = init.wasClean ?? false;
		}
	};
}

// Re-export protocol for consumers that need protocol types at runtime
let _protocol;
try {
	_protocol = require("@rivetkit/engine-envoy-protocol");
} catch {
	_protocol = {};
}
module.exports.protocol = _protocol;
module.exports.utils = {};

/**
 * Create a wrapped EnvoyHandle that matches the TS EnvoyHandle interface.
 */
function wrapHandle(jsHandle) {
	const handle = {
		started: () => jsHandle.started(),
		shutdown: (immediate) => jsHandle.shutdown(immediate ?? false),
		getProtocolMetadata: () => undefined,
		getEnvoyKey: () => jsHandle.envoyKey,
		getActor: (_actorId, _generation) => undefined,
		sleepActor: (actorId, generation) =>
			jsHandle.sleepActor(actorId, generation ?? null),
		stopActor: (actorId, generation, error) =>
			jsHandle.stopActor(actorId, generation ?? null, error ?? null),
		destroyActor: (actorId, generation) =>
			jsHandle.destroyActor(actorId, generation ?? null),
		setAlarm: (actorId, alarmTs, generation) =>
			jsHandle.setAlarm(actorId, alarmTs ?? null, generation ?? null),
		kvGet: async (actorId, keys) => {
			const bufKeys = keys.map((k) => Buffer.from(k));
			const result = await jsHandle.kvGet(actorId, bufKeys);
			return result.map((v) => (v ? new Uint8Array(v) : null));
		},
		kvPut: async (actorId, entries) => {
			const jsEntries = entries.map(([k, v]) => ({
				key: Buffer.from(k),
				value: Buffer.from(v),
			}));
			return jsHandle.kvPut(actorId, jsEntries);
		},
		kvDelete: async (actorId, keys) => {
			const bufKeys = keys.map((k) => Buffer.from(k));
			return jsHandle.kvDelete(actorId, bufKeys);
		},
		kvDeleteRange: async (actorId, start, end) => {
			return jsHandle.kvDeleteRange(
				actorId,
				Buffer.from(start),
				Buffer.from(end),
			);
		},
		kvListAll: async (actorId, options) => {
			const result = await jsHandle.kvListAll(actorId, options || null);
			return result.map((e) => [new Uint8Array(e.key), new Uint8Array(e.value)]);
		},
		kvListRange: async (actorId, start, end, exclusive, options) => {
			const result = await jsHandle.kvListRange(
				actorId,
				Buffer.from(start),
				Buffer.from(end),
				exclusive,
				options || null,
			);
			return result.map((e) => [new Uint8Array(e.key), new Uint8Array(e.value)]);
		},
		kvListPrefix: async (actorId, prefix, options) => {
			const result = await jsHandle.kvListPrefix(
				actorId,
				Buffer.from(prefix),
				options || null,
			);
			return result.map((e) => [new Uint8Array(e.key), new Uint8Array(e.value)]);
		},
		kvDrop: (actorId) => jsHandle.kvDrop(actorId),
		restoreHibernatingRequests: (actorId, metaEntries) => {
			const requests = (metaEntries || []).map((e) => ({
				gatewayId: Buffer.from(e.gatewayId),
				requestId: Buffer.from(e.requestId),
			}));
			jsHandle.restoreHibernatingRequests(actorId, requests);
		},
		sendHibernatableWebSocketMessageAck: (
			gatewayId,
			requestId,
			clientMessageIndex,
		) =>
			jsHandle.sendHibernatableWebSocketMessageAck(
				Buffer.from(gatewayId),
				Buffer.from(requestId),
				clientMessageIndex,
			),
		startServerlessActor: async (payload) =>
			await jsHandle.startServerless(Buffer.from(payload)),
		// Internal: expose raw handle for openDatabaseFromEnvoy
		_raw: jsHandle,
	};
	return handle;
}

/**
 * Start the native envoy synchronously with EnvoyConfig callbacks.
 * Returns a wrapped handle matching the TS EnvoyHandle interface.
 */
function startEnvoySync(config) {
	const wrappedHandle = { current: null };

	const jsHandle = native.startEnvoySyncJs(
		{
			endpoint: config.endpoint,
			token: config.token || "",
			namespace: config.namespace,
			poolName: config.poolName,
			version: config.version,
			metadata: config.metadata || null,
			notGlobal: config.notGlobal ?? false,
		},
		(event) => {
			handleEvent(event, config, wrappedHandle);
		},
	);

	const handle = wrapHandle(jsHandle);
	wrappedHandle.current = handle;
	return handle;
}

/**
 * Start the native envoy and wait for it to be ready.
 */
async function startEnvoy(config) {
	const handle = startEnvoySync(config);
	await handle.started();
	return handle;
}

/**
 * Open a native database backed by envoy KV.
 */
async function openDatabaseFromEnvoy(handle, actorId, preloadedEntries) {
	const rawHandle = handle._raw || handle;
	const nativePreloadedEntries = preloadedEntries
		? preloadedEntries.map(([key, value]) => ({
				key: Buffer.from(key),
				value: Buffer.from(value),
			}))
		: null;
	return native.openDatabaseFromEnvoy(
		rawHandle,
		actorId,
		nativePreloadedEntries,
	);
}

function decodePreloadedKv(preloadedKv) {
	if (!preloadedKv) {
		return null;
	}

	const decodeBytes = (value) => Uint8Array.from(Buffer.from(value, "base64"));

	return {
		entries: (preloadedKv.entries || []).map((entry) => ({
			key: decodeBytes(entry.key),
			value: decodeBytes(entry.value),
			metadata: {
				version: decodeBytes(entry.metadata.version),
				updateTs: entry.metadata.updateTs,
			},
		})),
		requestedGetKeys: (preloadedKv.requestedGetKeys || []).map(decodeBytes),
		requestedPrefixes: (preloadedKv.requestedPrefixes || []).map(decodeBytes),
	};
}

/**
 * Route callback envelopes from the native addon to EnvoyConfig callbacks.
 */
function handleEvent(event, config, wrappedHandle) {
	const handle = wrappedHandle.current;

	switch (event.kind) {
		case "actor_start": {
			const input = event.input ? Buffer.from(event.input, "base64") : undefined;
			const actorConfig = {
				name: event.name,
				key: event.key || undefined,
				createTs: event.createTs,
				input,
			};
			Promise.resolve(
				config.onActorStart(
					handle,
					event.actorId,
					event.generation,
					actorConfig,
					decodePreloadedKv(event.preloadedKv),
				),
			).then(
				async () => {
					if (handle._raw) {
						await handle._raw.respondCallback(event.responseId, {});
					}
				},
				async (err) => {
					console.error("onActorStart error:", err);
					if (handle._raw) {
						await handle._raw.respondCallback(event.responseId, {
							error: String(err),
						});
					}
				},
			);
			break;
		}
		case "actor_stop": {
			Promise.resolve(
				config.onActorStop(
					handle,
					event.actorId,
					event.generation,
					event.reason || "stopped",
				),
			).then(
				async () => {
					if (handle._raw) {
						await handle._raw.respondCallback(event.responseId, {});
					}
				},
				async (err) => {
					console.error("onActorStop error:", err);
					if (handle._raw) {
						await handle._raw.respondCallback(event.responseId, {
							error: String(err),
						});
					}
				},
			);
			break;
		}
		case "http_request": {
			const body = event.body ? Buffer.from(event.body, "base64") : undefined;
			const messageId = Buffer.from(event.messageId);
			const gatewayId = messageId.subarray(0, 4);
			const requestId = messageId.subarray(4, 8);

			// Build a Request object matching the TS envoy-client interface
			const headers = new Headers(event.headers || {});
			const url = `http://actor${event.path}`;
			const request = new Request(url, {
				method: event.method,
				headers,
				body: body || undefined,
			});

			Promise.resolve(
				config.fetch(handle, event.actorId, gatewayId, requestId, request),
			).then(
				async (response) => {
					if (handle._raw && response) {
						const respHeaders = {};
						if (response.headers) {
							response.headers.forEach((value, key) => {
								respHeaders[key] = value;
							});
						}
						const respBody = response.body
							? Buffer.from(await response.arrayBuffer()).toString("base64")
							: undefined;
						await handle._raw.respondCallback(event.responseId, {
							status: response.status || 200,
							headers: respHeaders,
							body: respBody,
						});
					}
				},
				async (err) => {
					console.error("fetch callback error:", err);
					if (handle._raw) {
						await handle._raw.respondCallback(event.responseId, {
							status: 500,
							headers: { "content-type": "text/plain" },
							body: Buffer.from(String(err)).toString("base64"),
						});
					}
				},
			);
			break;
		}
		case "websocket_open": {
			if (config.websocket) {
				const messageId = Buffer.from(event.messageId);
				const gatewayId = messageId.subarray(0, 4);
				const requestId = messageId.subarray(4, 8);
				const wsIdHex = gatewayId.toString("hex") + requestId.toString("hex");

				const headers = new Headers(event.headers || {});
				headers.set("Upgrade", "websocket");
				headers.set("Connection", "Upgrade");
				const url = `http://actor${event.path}`;
				const request = new Request(url, {
					method: "GET",
					headers,
				});

				// Create a WebSocket-like object backed by EventTarget.
				// The EngineActorDriver calls addEventListener on this.
				// Events are dispatched when native websocket_message/close events arrive.
				const target = new EventTarget();
				const OPEN = 1;
				const CLOSED = 3;
				const ws = Object.create(target, {
					readyState: { value: OPEN, writable: true },
					OPEN: { value: OPEN },
					CLOSED: { value: CLOSED },
					send: {
						value: (data) => {
							if (handle._raw) {
								const isBinary =
									data instanceof ArrayBuffer || ArrayBuffer.isView(data);
								const bytes = isBinary
									? Buffer.from(data instanceof ArrayBuffer ? data : data.buffer, data instanceof ArrayBuffer ? 0 : data.byteOffset, data instanceof ArrayBuffer ? data.byteLength : data.byteLength)
									: Buffer.from(String(data));
								handle._raw.sendWsMessage(gatewayId, requestId, bytes, isBinary);
							}
						}
					},
					close: {
						value: (code, reason) => {
							ws.readyState = CLOSED;
							if (handle._raw) {
								handle._raw.closeWebsocket(
									gatewayId,
									requestId,
									code != null ? code : undefined,
									reason != null ? String(reason) : undefined,
								);
							}
						}
					},
					addEventListener: { value: target.addEventListener.bind(target) },
					removeEventListener: { value: target.removeEventListener.bind(target) },
					dispatchEvent: { value: target.dispatchEvent.bind(target) },
				});

				// Store the ws object so websocket_message/close events can dispatch to it
				if (!handle._wsMap) handle._wsMap = new Map();
				handle._wsMap.set(wsIdHex, ws);

				const canHibernate = config.hibernatableWebSocket
					? config.hibernatableWebSocket.canHibernate(
						event.actorId,
						gatewayId,
						requestId,
						request,
					)
					: false;

				Promise.resolve(
					config.websocket(
						handle,
						event.actorId,
						ws,
						gatewayId,
						requestId,
						request,
						event.path,
						event.headers || {},
						canHibernate,
						false,
					),
				).then(() => {
					ws.dispatchEvent(new Event("open"));
				}).catch((err) => {
					console.error("[wrapper] websocket callback error:", err);
				});
			}
			break;
		}
		case "websocket_message": {
			if (handle._wsMap && event.messageId) {
				const messageId = Buffer.from(event.messageId);
				const gatewayId = messageId.subarray(0, 4);
				const requestId = messageId.subarray(4, 8);
				const wsIdHex = gatewayId.toString("hex") + requestId.toString("hex");

				const ws = handle._wsMap.get(wsIdHex);

				if (ws) {
					const data = event.data
						? (event.binary
							? Buffer.from(event.data, "base64")
							: Buffer.from(event.data, "base64").toString())
						: "";
					const msgEvent = new MessageEvent("message", { data });
					msgEvent.rivetGatewayId = messageId.subarray(0, 4);
					msgEvent.rivetRequestId = messageId.subarray(4, 8);
					msgEvent.rivetMessageIndex = messageId.readUint16LE(8);
					ws.dispatchEvent(msgEvent);
				}
			}
			break;
		}
		case "websocket_close": {
			if (handle._wsMap && event.messageId) {
				const messageId = Buffer.from(event.messageId);
				const gatewayId = messageId.subarray(0, 4);
				const requestId = messageId.subarray(4, 8);
				const wsIdHex = gatewayId.toString("hex") + requestId.toString("hex");

				const ws = handle._wsMap.get(wsIdHex);
				if (ws) {
					ws.readyState = 3;
					ws.dispatchEvent(new CloseEvent("close", {
						code: event.code || 1000,
						reason: event.reason || "",
					}));
					handle._wsMap.delete(wsIdHex);
				}
			}
			break;
		}
		case "hibernation_restore":
		case "alarm":
		case "wake":
			break;
		case "shutdown": {
			if (config.onShutdown) {
				config.onShutdown();
			}
			break;
		}
		default:
			console.warn("unknown native event kind:", event.kind);
	}
}

module.exports.startEnvoy = startEnvoy;
module.exports.startEnvoySync = startEnvoySync;
module.exports.openDatabaseFromEnvoy = openDatabaseFromEnvoy;
