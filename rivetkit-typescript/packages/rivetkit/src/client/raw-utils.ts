import {
	HEADER_CONN_PARAMS,
	HEADER_ORIGINAL_REQUEST_URL,
	PATH_WEBSOCKET_PREFIX,
} from "@/common/actor-router-consts";
import { isRequestLike, isUrlLike } from "@/common/fetch-like";
import { deconstructError } from "@/common/utils";
import type {
	EngineControlClient,
	GatewayRequestOptions,
	GatewayTarget,
} from "@/engine-client/driver";
import { ActorError } from "./errors";
import { logger } from "./log";
import { addOutboundTelemetryHeaders } from "./outbound-telemetry";

/** Buffer a one-shot ReadableStream body to bytes so every retry attempt can re-send it. */
export async function prepareRetryableInit(
	init: RequestInit,
): Promise<RequestInit> {
	if (init.body instanceof ReadableStream) {
		return {
			...init,
			body: await bufferReadableStream(init.body, init.signal),
		};
	}

	return init;
}

async function bufferReadableStream(
	body: ReadableStream,
	signal?: AbortSignal | null,
): Promise<Uint8Array> {
	signal?.throwIfAborted();

	const reader = body.getReader();
	const chunks: Uint8Array[] = [];
	let byteLength = 0;
	const onAbort = () => {
		void reader.cancel(signal?.reason).catch(() => {
			// The abort reason takes precedence over a stream cancellation error.
		});
	};
	signal?.addEventListener("abort", onAbort, { once: true });

	try {
		while (true) {
			const { done, value } = await reader.read();
			signal?.throwIfAborted();
			if (done) {
				break;
			}
			if (!(value instanceof Uint8Array)) {
				throw new TypeError(
					"Received a non-Uint8Array chunk from a request body stream.",
				);
			}
			chunks.push(value);
			byteLength += value.byteLength;
		}
	} finally {
		signal?.removeEventListener("abort", onAbort);
		reader.releaseLock();
	}

	const buffered = new Uint8Array(byteLength);
	let offset = 0;
	for (const chunk of chunks) {
		buffered.set(chunk, offset);
		offset += chunk.byteLength;
	}
	return buffered;
}

/**
 * Shared implementation for raw HTTP fetch requests
 */
export async function rawHttpFetch(
	driver: EngineControlClient,
	target: GatewayTarget,
	params: unknown,
	input: string | URL | Request,
	init?: RequestInit,
	options: GatewayRequestOptions = {},
	telemetryHeaders: Record<string, string> = {},
): Promise<Response> {
	// Extract path and merge init options
	let path: string;
	let originalUrl: string | undefined;
	let mergedInit: RequestInit = init || {};

	if (typeof input === "string") {
		path = input;
	} else if (isUrlLike(input)) {
		path = input.pathname + input.search;
		originalUrl = input.toString();
	} else if (isRequestLike(input)) {
		// Extract path from Request URL
		const url = new URL(input.url);
		path = url.pathname + url.search;
		originalUrl = url.toString();
		// Merge Request properties with init
		const requestHeaders = new Headers(input.headers);
		const initHeaders = new Headers(init?.headers || {});

		// Merge headers - init headers override request headers
		const mergedHeaders = new Headers(requestHeaders);
		initHeaders.forEach((value, key) => {
			mergedHeaders.set(key, value);
		});

		mergedInit = {
			method: input.method,
			body: input.body,
			mode: input.mode,
			credentials: input.credentials,
			redirect: input.redirect,
			referrer: input.referrer,
			referrerPolicy: input.referrerPolicy,
			integrity: input.integrity,
			keepalive: input.keepalive,
			signal: input.signal,
			...mergedInit, // init overrides Request properties
			headers: mergedHeaders, // headers must be set after spread to ensure proper merge
		};
		// Add duplex if body is present
		if (mergedInit.body) {
			(mergedInit as any).duplex = "half";
		}
	} else {
		throw new TypeError("Invalid input type for fetch");
	}

	try {
		logger().debug(
			"directId" in target
				? {
						msg: "sending raw http request to actor",
						actorId: target.directId,
					}
				: {
						msg: "sending raw http request with actor query",
						query: target,
					},
		);

		// Build the URL with normalized path
		const normalizedPath = path.startsWith("/") ? path.slice(1) : path;
		const url = new URL(`http://actor/request/${normalizedPath}`);

		// Forward conn params if provided
		const proxyRequestHeaders = new Headers(mergedInit.headers);
		proxyRequestHeaders.delete(HEADER_ORIGINAL_REQUEST_URL);
		if (originalUrl) {
			proxyRequestHeaders.set(HEADER_ORIGINAL_REQUEST_URL, originalUrl);
		}
		if (params) {
			proxyRequestHeaders.set(HEADER_CONN_PARAMS, JSON.stringify(params));
		}
		addOutboundTelemetryHeaders(proxyRequestHeaders, telemetryHeaders);

		// Forward the request to the actor
		const proxyRequest = new Request(url, {
			...mergedInit,
			headers: proxyRequestHeaders,
		});

		return driver.sendRequest(target, proxyRequest, options);
	} catch (err) {
		// Standardize to ClientActorError instead of the native backend error
		const { group, code, message, metadata, rayId, actor } =
			deconstructError(err, true);
		throw new ActorError(group, code, message, { metadata, rayId, actor });
	}
}

/**
 * Shared implementation for raw WebSocket connections
 */
export async function rawWebSocket(
	driver: EngineControlClient,
	target: GatewayTarget,
	params: unknown,
	path?: string,
	// TODO: Supportp rotocols
	_protocols?: string | string[],
	options: GatewayRequestOptions = {},
): Promise<any> {
	// TODO: Do we need encoding in rawWebSocket?
	const encoding = "bare";

	// Parse path and query parameters
	let pathPortion = "";
	let queryPortion = "";
	if (path) {
		const queryIndex = path.indexOf("?");
		if (queryIndex !== -1) {
			pathPortion = path.substring(0, queryIndex);
			queryPortion = path.substring(queryIndex); // includes the '?'
		} else {
			pathPortion = path;
		}
		// Remove leading slash if present
		if (pathPortion.startsWith("/")) {
			pathPortion = pathPortion.slice(1);
		}
	}

	const fullPath = `${PATH_WEBSOCKET_PREFIX}${pathPortion}${queryPortion}`;

	logger().debug({
		msg: "opening websocket",
		target,
		encoding,
		path: fullPath,
	});

	// Open WebSocket
	const ws = await driver.openWebSocket(
		fullPath,
		target,
		encoding,
		params,
		options,
	);

	// Node & browser WebSocket types are incompatible
	return ws as any;
}
