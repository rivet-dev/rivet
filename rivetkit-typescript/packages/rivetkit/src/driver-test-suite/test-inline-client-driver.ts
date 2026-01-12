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
import { type DeconstructedError, noopNext } from "@/common/utils";
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
import type { GetUpgradeWebSocket } from "@/utils";
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
	let getUpgradeWebSocket: GetUpgradeWebSocket;
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
			protocols.push(
				`${WS_PROTOCOL_ACTOR}${encodeURIComponent(actorId)}`,
			);
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
			c: HonoContext,
			path: string,
			actorId: string,
			encoding: Encoding,
			params: unknown,
		): Promise<Response> {
			const upgradeWebSocket = getUpgradeWebSocket?.();
			invariant(upgradeWebSocket, "missing getUpgradeWebSocket");

			const wsHandler = this.openWebSocket(
				path,
				actorId,
				encoding,
				params,
			);
			return upgradeWebSocket(() => wsHandler)(c, noopNext());
		},
		displayInformation(): ManagerDisplayInformation {
			return { properties: {} };
		},
		setGetUpgradeWebSocket: (getUpgradeWebSocketInner) => {
			getUpgradeWebSocket = getUpgradeWebSocketInner;
		},
		kvGet: (_actorId: string, _key: Uint8Array) => {
			throw new Error("kvGet not impelmented on inline client driver");
		},
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
