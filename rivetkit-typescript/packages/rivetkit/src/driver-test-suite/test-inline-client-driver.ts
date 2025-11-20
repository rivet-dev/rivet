import * as cbor from "cbor-x";
import type { Context as HonoContext } from "hono";
import invariant from "invariant";
import type { WebSocket } from "ws";
import type { Encoding } from "@/actor/protocol/serde";
import { assertUnreachable } from "@/actor/utils";
import { ActorError as ClientActorError } from "@/client/errors";
import {
	HEADER_ACTOR_QUERY,
	HEADER_CONN_PARAMS,
	HEADER_ENCODING,
	WS_PROTOCOL_ACTOR,
	WS_PROTOCOL_CONN_PARAMS,
	WS_PROTOCOL_ENCODING,
	WS_PROTOCOL_STANDARD,
	WS_PROTOCOL_TARGET,
	WS_TEST_PROTOCOL_PATH,
} from "@/common/actor-router-consts";
import type { UniversalEventSource } from "@/common/eventsource-interface";
import type { DeconstructedError } from "@/common/utils";
import { importWebSocket } from "@/common/websocket";
import {
	type ActorOutput,
	type CreateInput,
	type GetForIdInput,
	type GetOrCreateWithKeyInput,
	type GetWithKeyInput,
	HEADER_ACTOR_ID,
	type ListActorsInput,
	type ManagerDisplayInformation,
	type ManagerDriver,
} from "@/driver-helpers/mod";
import type { ActorQuery } from "@/manager/protocol/query";
import type { UniversalWebSocket } from "@/mod";
import type * as protocol from "@/schemas/client-protocol/mod";
import { logger } from "./log";

export interface TestInlineDriverCallRequest {
	encoding: Encoding;
	method: string;
	args: unknown[];
}

export type TestInlineDriverCallResponse<T> =
	| {
			ok: T;
	  }
	| {
			err: DeconstructedError;
	  };

/**
 * Creates a client driver used for testing the inline client driver. This will send a request to the HTTP server which will then internally call the internal client and return the response.
 */
export function createTestInlineClientDriver(
	endpoint: string,
	encoding: Encoding,
): ManagerDriver {
	return {
		getForId(input: GetForIdInput): Promise<ActorOutput | undefined> {
			return makeInlineRequest(endpoint, encoding, "getForId", [input]);
		},
		getWithKey(input: GetWithKeyInput): Promise<ActorOutput | undefined> {
			return makeInlineRequest(endpoint, encoding, "getWithKey", [input]);
		},
		getOrCreateWithKey(
			input: GetOrCreateWithKeyInput,
		): Promise<ActorOutput> {
			return makeInlineRequest(endpoint, encoding, "getOrCreateWithKey", [
				input,
			]);
		},
		createActor(input: CreateInput): Promise<ActorOutput> {
			return makeInlineRequest(endpoint, encoding, "createActor", [
				input,
			]);
		},
		listActors(input: ListActorsInput): Promise<ActorOutput[]> {
			return makeInlineRequest(endpoint, encoding, "listActors", [input]);
		},
		async sendRequest(
			actorId: string,
			actorRequest: Request,
		): Promise<Response> {
			// Normalize path to match other drivers
			const oldUrl = new URL(actorRequest.url);
			const normalizedPath = oldUrl.pathname.startsWith("/")
				? oldUrl.pathname.slice(1)
				: oldUrl.pathname;
			const pathWithQuery = normalizedPath + oldUrl.search;

			logger().debug({
				msg: "sending raw http request via test inline driver",
				actorId,
				encoding,
				path: pathWithQuery,
			});

			// Use the dedicated raw HTTP endpoint
			const url = `${endpoint}/.test/inline-driver/send-request/${pathWithQuery}`;

			logger().debug({
				msg: "rewriting http url",
				from: oldUrl,
				to: url,
			});

			// Merge headers with our metadata
			const headers = new Headers(actorRequest.headers);
			headers.set(HEADER_ACTOR_ID, actorId);

			// Forward the request directly
			const response = await fetch(
				new Request(url, {
					method: actorRequest.method,
					headers,
					body: actorRequest.body,
					signal: actorRequest.signal,
					duplex: "half",
				} as RequestInit),
			);

			// Check if it's an error response from our handler
			if (
				!response.ok &&
				response.headers
					.get("content-type")
					?.includes("application/json")
			) {
				try {
					// Clone the response to avoid consuming the body
					const clonedResponse = response.clone();
					const errorData = (await clonedResponse.json()) as any;
					if (errorData.error) {
						// Handle both error formats:
						// 1. { error: { code, message, metadata } } - structured format
						// 2. { error: "message" } - simple string format (from custom onRequest handlers)
						if (typeof errorData.error === "object") {
							throw new ClientActorError(
								errorData.error.code,
								errorData.error.message,
								errorData.error.metadata,
							);
						}
						// For simple string errors, just return the response as-is
						// This allows custom onRequest handlers to return their own error formats
					}
				} catch (e) {
					// If it's not our error format, just return the response as-is
					if (!(e instanceof ClientActorError)) {
						return response;
					}
					throw e;
				}
			}

			return response;
		},
		async openWebSocket(
			path: string,
			actorId: string,
			encoding: Encoding,
			params: unknown,
		): Promise<UniversalWebSocket> {
			const WebSocket = await importWebSocket();

			// Normalize path to match other drivers
			const normalizedPath = path.startsWith("/") ? path.slice(1) : path;

			// Create WebSocket connection to the test endpoint
			const wsUrl = new URL(
				`${endpoint}/.test/inline-driver/connect-websocket/ws`,
			);

			logger().debug({
				msg: "creating websocket connection via test inline driver",
				url: wsUrl.toString(),
			});

			// Convert http/https to ws/wss
			const wsProtocol = wsUrl.protocol === "https:" ? "wss:" : "ws:";
			const finalWsUrl = `${wsProtocol}//${wsUrl.host}${wsUrl.pathname}`;

			// Build protocols for the connection
			const protocols: string[] = [];
			protocols.push(WS_PROTOCOL_STANDARD);
			protocols.push(`${WS_PROTOCOL_TARGET}actor`);
			protocols.push(`${WS_PROTOCOL_ACTOR}${encodeURIComponent(actorId)}`);
			protocols.push(`${WS_PROTOCOL_ENCODING}${encoding}`);
			protocols.push(
				`${WS_TEST_PROTOCOL_PATH}${encodeURIComponent(normalizedPath)}`,
			);
			if (params !== undefined) {
				protocols.push(
					`${WS_PROTOCOL_CONN_PARAMS}${encodeURIComponent(JSON.stringify(params))}`,
				);
			}

			logger().debug({
				msg: "connecting to websocket",
				url: finalWsUrl,
				protocols,
			});

			// Create and return the WebSocket
			// Node & browser WebSocket types are incompatible
			const ws = new WebSocket(finalWsUrl, protocols) as any;

			return ws;
		},
		async proxyRequest(
			c: HonoContext,
			actorRequest: Request,
			actorId: string,
		): Promise<Response> {
			return await this.sendRequest(actorId, actorRequest);
		},
		proxyWebSocket(
			_c: HonoContext,
			_path: string,
			_actorId: string,
			_encoding: Encoding,
			_params: unknown,
		): Promise<Response> {
			throw "UNIMPLEMENTED";
			// const upgradeWebSocket = this.#runConfig.getUpgradeWebSocket?.();
			// invariant(upgradeWebSocket, "missing getUpgradeWebSocket");
			//
			// const wsHandler = this.openWebSocket(path, actorId, encoding, connParams);
			// return upgradeWebSocket(() => wsHandler)(c, noopNext());
		},
		displayInformation(): ManagerDisplayInformation {
			return { name: "Test Inline", properties: {} };
		},
		// TODO:
		getOrCreateInspectorAccessToken: () => "",

		// action: async <Args extends Array<unknown> = unknown[], Response = unknown>(
		// 	_c: HonoContext | undefined,
		// 	actorQuery: ActorQuery,
		// 	encoding: Encoding,
		// 	params: unknown,
		// 	name: string,
		// 	args: Args,
		// ): Promise<Response> => {
		// 	return makeInlineRequest<Response>(
		// 		endpoint,
		// 		encoding,
		// 		"action",
		// 		[undefined, actorQuery, encoding, params, name, args],
		// 	);
		// },
		//
		// resolveActorId: async (
		// 	_c: HonoContext | undefined,
		// 	actorQuery: ActorQuery,
		// 	encodingKind: Encoding,
		// 	params: unknown,
		// ): Promise<string> => {
		// 	return makeInlineRequest<string>(
		// 		endpoint,
		// 		encodingKind,
		// 		"resolveActorId",
		// 		[undefined, actorQuery, encodingKind, params],
		// 	);
		// },
		//
		// connectWebSocket: async (
		// 	_c: HonoContext | undefined,
		// 	actorQuery: ActorQuery,
		// 	encodingKind: Encoding,
		// 	params: unknown,
		// ): Promise<WebSocket> => {
		// 	const WebSocket = await importWebSocket();
		//
		// 	logger().debug({
		// 		msg: "creating websocket connection via test inline driver",
		// 		actorQuery,
		// 		encodingKind,
		// 	});
		//
		// 	// Create WebSocket connection to the test endpoint
		// 	const wsUrl = new URL(
		// 		`${endpoint}/registry/.test/inline-driver/connect-websocket`,
		// 	);
		// 	wsUrl.searchParams.set("actorQuery", JSON.stringify(actorQuery));
		// 	if (params !== undefined)
		// 		wsUrl.searchParams.set("params", JSON.stringify(params));
		// 	wsUrl.searchParams.set("encodingKind", encodingKind);
		//
		// 	// Convert http/https to ws/wss
		// 	const wsProtocol = wsUrl.protocol === "https:" ? "wss:" : "ws:";
		// 	const finalWsUrl = `${wsProtocol}//${wsUrl.host}${wsUrl.pathname}${wsUrl.search}`;
		//
		// 	logger().debug({ msg: "connecting to websocket", url: finalWsUrl });
		//
		// 	// Create and return the WebSocket
		// 	// Node & browser WebSocket types are incompatible
		// 	const ws = new WebSocket(finalWsUrl, [
		// 		// HACK: See packages/drivers/cloudflare-workers/src/websocket.ts
		// 		"rivetkit",
		// 	]) as any;
		//
		// 	return ws;
		// },
		//
		// connectSse: async (
		// 	_c: HonoContext | undefined,
		// 	actorQuery: ActorQuery,
		// 	encodingKind: Encoding,
		// 	params: unknown,
		// ): Promise<UniversalEventSource> => {
		// 	logger().debug({
		// 		msg: "creating sse connection via test inline driver",
		// 		actorQuery,
		// 		encodingKind,
		// 		params,
		// 	});
		//
		// 	// Dynamically import EventSource if needed
		// 	const EventSourceImport = await import("eventsource");
		// 	// Handle both ES modules (default) and CommonJS export patterns
		// 	const EventSourceConstructor =
		// 		(EventSourceImport as any).default || EventSourceImport;
		//
		// 	// Encode parameters for the URL
		// 	const actorQueryParam = encodeURIComponent(JSON.stringify(actorQuery));
		// 	const encodingParam = encodeURIComponent(encodingKind);
		// 	const paramsParam = params
		// 		? encodeURIComponent(JSON.stringify(params))
		// 		: null;
		//
		// 	// Create SSE connection URL
		// 	const sseUrl = new URL(
		// 		`${endpoint}/registry/.test/inline-driver/connect-sse`,
		// 	);
		// 	sseUrl.searchParams.set("actorQueryRaw", actorQueryParam);
		// 	sseUrl.searchParams.set("encodingKind", encodingParam);
		// 	if (paramsParam) {
		// 		sseUrl.searchParams.set("params", paramsParam);
		// 	}
		//
		// 	logger().debug({ msg: "connecting to sse", url: sseUrl.toString() });
		//
		// 	// Create and return the EventSource
		// 	const eventSource = new EventSourceConstructor(sseUrl.toString());
		//
		// 	// Wait for the connection to be established before returning
		// 	await new Promise<void>((resolve, reject) => {
		// 		eventSource.onopen = () => {
		// 			logger().debug({ msg: "sse connection established" });
		// 			resolve();
		// 		};
		//
		// 		eventSource.onerror = (event: Event) => {
		// 			logger().error({ msg: "sse connection failed", event });
		// 			reject(new Error("Failed to establish SSE connection"));
		// 		};
		//
		// 		// Set a timeout in case the connection never establishes
		// 		setTimeout(() => {
		// 			if (eventSource.readyState !== EventSourceConstructor.OPEN) {
		// 				reject(new Error("SSE connection timed out"));
		// 			}
		// 		}, 10000); // 10 second timeout
		// 	});
		//
		// 	return eventSource as UniversalEventSource;
		// },
		//
		// sendHttpMessage: async (
		// 	_c: HonoContext | undefined,
		// 	actorId: string,
		// 	encoding: Encoding,
		// 	connectionId: string,
		// 	connectionToken: string,
		// 	message: protocol.ToServer,
		// ): Promise<void> => {
		// 	logger().debug({
		// 		msg: "sending http message via test inline driver",
		// 		actorId,
		// 		encoding,
		// 		connectionId,
		// 	});
		//
		// 	const result = await fetch(
		// 		`${endpoint}/registry/.test/inline-driver/call`,
		// 		{
		// 			method: "POST",
		// 			headers: {
		// 				"Content-Type": "application/json",
		// 			},
		// 			body: JSON.stringify({
		// 				encoding,
		// 				method: "sendHttpMessage",
		// 				args: [
		// 					undefined,
		// 					actorId,
		// 					encoding,
		// 					connectionId,
		// 					connectionToken,
		// 					message,
		// 				],
		// 			} satisfies TestInlineDriverCallRequest),
		// 		},
		// 	);
		//
		// 	if (!result.ok) {
		// 		throw new Error(`Failed to send HTTP message: ${result.statusText}`);
		// 	}
		//
		// 	// Discard response
		// 	await result.body?.cancel();
		// },
		//
		// rawHttpRequest: async (
		// 	_c: HonoContext | undefined,
		// 	actorQuery: ActorQuery,
		// 	encoding: Encoding,
		// 	params: unknown,
		// 	path: string,
		// 	init: RequestInit,
		// ): Promise<Response> => {
		// 	// Normalize path to match other drivers
		// 	const normalizedPath = path.startsWith("/") ? path.slice(1) : path;
		//
		// 	logger().debug({
		// 		msg: "sending raw http request via test inline driver",
		// 		actorQuery,
		// 		encoding,
		// 		path: normalizedPath,
		// 	});
		//
		// 	// Use the dedicated raw HTTP endpoint
		// 	const url = `${endpoint}/registry/.test/inline-driver/raw-http/${normalizedPath}`;
		//
		// 	logger().debug({ msg: "rewriting http url", from: path, to: url });
		//
		// 	// Merge headers with our metadata
		// 	const headers = new Headers(init.headers);
		// 	headers.set(HEADER_ACTOR_QUERY, JSON.stringify(actorQuery));
		// 	headers.set(HEADER_ENCODING, encoding);
		// 	if (params !== undefined) {
		// 		headers.set(HEADER_CONN_PARAMS, JSON.stringify(params));
		// 	}
		//
		// 	// Forward the request directly
		// 	const response = await fetch(url, {
		// 		...init,
		// 		headers,
		// 	});
		//
		// 	// Check if it's an error response from our handler
		// 	if (
		// 		!response.ok &&
		// 		response.headers.get("content-type")?.includes("application/json")
		// 	) {
		// 		try {
		// 			// Clone the response to avoid consuming the body
		// 			const clonedResponse = response.clone();
		// 			const errorData = (await clonedResponse.json()) as any;
		// 			if (errorData.error) {
		// 				// Handle both error formats:
		// 				// 1. { error: { code, message, metadata } } - structured format
		// 				// 2. { error: "message" } - simple string format (from custom onRequest handlers)
		// 				if (typeof errorData.error === "object") {
		// 					throw new ClientActorError(
		// 						errorData.error.code,
		// 						errorData.error.message,
		// 						errorData.error.metadata,
		// 					);
		// 				}
		// 				// For simple string errors, just return the response as-is
		// 				// This allows custom onRequest handlers to return their own error formats
		// 			}
		// 		} catch (e) {
		// 			// If it's not our error format, just return the response as-is
		// 			if (!(e instanceof ClientActorError)) {
		// 				return response;
		// 			}
		// 			throw e;
		// 		}
		// 	}
		//
		// 	return response;
		// },
		//
		// rawWebSocket: async (
		// 	_c: HonoContext | undefined,
		// 	actorQuery: ActorQuery,
		// 	encoding: Encoding,
		// 	params: unknown,
		// 	path: string,
		// 	protocols: string | string[] | undefined,
		// ): Promise<WebSocket> => {
		// 	logger().debug({ msg: "test inline driver rawWebSocket called" });
		// 	const WebSocket = await importWebSocket();
		//
		// 	// Normalize path to match other drivers
		// 	const normalizedPath = path.startsWith("/") ? path.slice(1) : path;
		//
		// 	logger().debug({
		// 		msg: "creating raw websocket connection via test inline driver",
		// 		actorQuery,
		// 		encoding,
		// 		path: normalizedPath,
		// 		protocols,
		// 	});
		//
		// 	// Create WebSocket connection to the test endpoint
		// 	const wsUrl = new URL(
		// 		`${endpoint}/registry/.test/inline-driver/raw-websocket`,
		// 	);
		// 	wsUrl.searchParams.set("actorQuery", JSON.stringify(actorQuery));
		// 	if (params !== undefined)
		// 		wsUrl.searchParams.set("params", JSON.stringify(params));
		// 	wsUrl.searchParams.set("encodingKind", encoding);
		// 	wsUrl.searchParams.set("path", normalizedPath);
		// 	if (protocols !== undefined)
		// 		wsUrl.searchParams.set("protocols", JSON.stringify(protocols));
		//
		// 	// Convert http/https to ws/wss
		// 	const wsProtocol = wsUrl.protocol === "https:" ? "wss:" : "ws:";
		// 	const finalWsUrl = `${wsProtocol}//${wsUrl.host}${wsUrl.pathname}${wsUrl.search}`;
		//
		// 	logger().debug({ msg: "connecting to raw websocket", url: finalWsUrl });
		//
		// 	logger().debug({
		// 		msg: "rewriting websocket url",
		// 		from: path,
		// 		to: finalWsUrl,
		// 	});
		//
		// 	// Create and return the WebSocket
		// 	// Node & browser WebSocket types are incompatible
		// 	const ws = new WebSocket(finalWsUrl, [
		// 		// HACK: See packages/drivers/cloudflare-workers/src/websocket.ts
		// 		"rivetkit",
		// 	]) as any;
		//
		// 	logger().debug({
		// 		msg: "test inline driver created websocket",
		// 		readyState: ws.readyState,
		// 		url: ws.url,
		// 	});
		//
		// 	return ws;
		// },
	} satisfies ManagerDriver;
}

async function makeInlineRequest<T>(
	endpoint: string,
	encoding: Encoding,
	method: string,
	args: unknown[],
): Promise<T> {
	logger().debug({
		msg: "sending inline request",
		encoding,
		method,
		args,
	});

	// Call driver
	const response = await fetch(`${endpoint}/.test/inline-driver/call`, {
		method: "POST",
		headers: {
			"Content-Type": "application/json",
		},
		body: cbor.encode({
			encoding,
			method,
			args,
		} satisfies TestInlineDriverCallRequest),
		duplex: "half",
	} as RequestInit);

	if (!response.ok) {
		throw new Error(
			`Failed to call inline ${method}: ${response.statusText}`,
		);
	}

	// Parse response
	const buffer = await response.arrayBuffer();
	const callResponse: TestInlineDriverCallResponse<T> = cbor.decode(
		new Uint8Array(buffer),
	);

	// Throw or OK
	if ("ok" in callResponse) {
		return callResponse.ok;
	} else if ("err" in callResponse) {
		throw new ClientActorError(
			callResponse.err.group,
			callResponse.err.code,
			callResponse.err.message,
			callResponse.err.metadata,
		);
	} else {
		assertUnreachable(callResponse);
	}
}
