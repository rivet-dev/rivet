import invariant from "invariant";
import { PATH_WEBSOCKET_PREFIX } from "@/common/actor-router-consts";
import { deconstructError } from "@/common/utils";
import { HEADER_CONN_PARAMS, type ManagerDriver } from "@/driver-helpers/mod";
import type { ActorQuery } from "@/manager/protocol/query";
import { queryActor } from "./actor-query";
import { ActorError } from "./errors";
import { logger } from "./log";

/**
 * Shared implementation for raw HTTP fetch requests
 */
export async function rawHttpFetch(
	driver: ManagerDriver,
	actorQuery: ActorQuery,
	params: unknown,
	input: string | URL | Request,
	init?: RequestInit,
): Promise<Response> {
	// Extract path and merge init options
	let path: string;
	let mergedInit: RequestInit = init || {};

	if (typeof input === "string") {
		path = input;
	} else if (input instanceof URL) {
		path = input.pathname + input.search;
	} else if (input instanceof Request) {
		// Extract path from Request URL
		const url = new URL(input.url);
		path = url.pathname + url.search;
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
		// Get the actor ID
		const { actorId } = await queryActor(undefined, actorQuery, driver);
		logger().debug({ msg: "found actor for raw http", actorId });
		invariant(actorId, "Missing actor ID");

		// Build the URL with normalized path
		const normalizedPath = path.startsWith("/") ? path.slice(1) : path;
		const url = new URL(`http://actor/request/${normalizedPath}`);

		// Forward conn params if provided
		const proxyRequestHeaders = new Headers(mergedInit.headers);
		if (params) {
			proxyRequestHeaders.set(HEADER_CONN_PARAMS, JSON.stringify(params));
		}

		// Forward the request to the actor
		const proxyRequest = new Request(url, {
			...mergedInit,
			headers: proxyRequestHeaders,
		});

		return driver.sendRequest(actorId, proxyRequest);
	} catch (err) {
		// Standardize to ClientActorError instead of the native backend error
		const { group, code, message, metadata } = deconstructError(
			err,
			logger(),
			{},
			true,
		);
		throw new ActorError(group, code, message, metadata);
	}
}

/**
 * Shared implementation for raw WebSocket connections
 */
export async function rawWebSocket(
	driver: ManagerDriver,
	actorQuery: ActorQuery,
	params: unknown,
	path?: string,
	// TODO: Supportp rotocols
	protocols?: string | string[],
): Promise<any> {
	// TODO: Do we need encoding in rawWebSocket?
	const encoding = "bare";

	// Get the actor ID
	const { actorId } = await queryActor(undefined, actorQuery, driver);
	logger().debug({ msg: "found actor for action", actorId });
	invariant(actorId, "Missing actor ID");

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
		actorId,
		encoding,
		path: fullPath,
	});

	// Open WebSocket
	const ws = await driver.openWebSocket(fullPath, actorId, encoding, params);

	// Node & browser WebSocket types are incompatible
	return ws as any;
}
