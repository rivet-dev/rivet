import * as cbor from "cbor-x";
import type { Context as HonoContext, Next } from "hono";
import type { Encoding } from "@/actor/protocol/serde";
import {
	getRequestEncoding,
	getRequestExposeInternalError,
} from "@/actor/router-endpoints";
import {
	buildActorNames,
	type RegistryConfig,
} from "@/registry/config";
import type * as protocol from "@/schemas/client-protocol/mod";
import {
	CURRENT_VERSION as CLIENT_PROTOCOL_CURRENT_VERSION,
	HTTP_RESPONSE_ERROR_VERSIONED,
} from "@/schemas/client-protocol/versioned";
import {
	type HttpResponseError as HttpResponseErrorJson,
	HttpResponseErrorSchema,
} from "@/schemas/client-protocol-zod/mod";
import { encodingIsBinary, serializeWithEncoding } from "@/serde";
import { bufferToArrayBuffer, VERSION } from "@/utils";
import { getLogHeaders } from "@/utils/env-vars";
import { getLogger, type Logger } from "./log";
import { deconstructError, stringifyError } from "./utils";

export function logger() {
	return getLogger("router");
}

export function loggerMiddleware(logger: Logger) {
	return async (c: HonoContext, next: Next) => {
		const method = c.req.method;
		const path = c.req.path;
		const startTime = Date.now();

		await next();

		const duration = Date.now() - startTime;
		logger.debug({
			msg: "http request",
			method,
			path,
			status: c.res.status,
			dt: `${duration}ms`,
			reqSize: c.req.header("content-length"),
			resSize: c.res.headers.get("content-length"),
			userAgent: c.req.header("user-agent"),
			...(getLogHeaders()
				? { allHeaders: JSON.stringify(c.req.header()) }
				: {}),
		});
	};
}

export function handleRouteNotFound(c: HonoContext) {
	return c.text("Not Found (RivetKit)", 404);
}

export function handleRouteError(error: unknown, c: HonoContext) {
	const exposeInternalError = getRequestExposeInternalError(c.req.raw);

	const { statusCode, group, code, message, metadata } = deconstructError(
		error,
		logger(),
		{
			method: c.req.method,
			path: c.req.path,
		},
		exposeInternalError,
	);

	let encoding: Encoding;
	try {
		encoding = getRequestEncoding(c.req);
	} catch (_) {
		encoding = "json";
	}

	const errorData = { group, code, message, metadata };
	const output = serializeWithEncoding(
		encoding,
		errorData,
		HTTP_RESPONSE_ERROR_VERSIONED,
		CLIENT_PROTOCOL_CURRENT_VERSION,
		HttpResponseErrorSchema,
		// JSON: metadata is the raw value (will be serialized by jsonStringifyCompat)
		(value): HttpResponseErrorJson => ({
			group: value.group,
			code: value.code,
			message: value.message,
			metadata: value.metadata,
		}),
		// BARE/CBOR: metadata needs to be CBOR-encoded to ArrayBuffer
		(value): protocol.HttpResponseError => ({
			group: value.group,
			code: value.code,
			message: value.message,
			metadata: value.metadata
				? bufferToArrayBuffer(cbor.encode(value.metadata))
				: null,
		}),
	);

	// TODO: Remove any
	return c.body(output as any, { status: statusCode });
}

export type MetadataRunnerKind =
	| { serverless: Record<never, never> }
	| { normal: Record<never, never> };

/**
 * Metadata response interface for the /metadata endpoint
 */
export interface MetadataResponse {
	runtime: string;
	version: string;
	runner?: {
		kind: MetadataRunnerKind;
	};
	actorNames: ReturnType<typeof buildActorNames>;
	/**
	 * Endpoint that the client should connect to to access this runner.
	 *
	 * If defined, will override the endpoint the user has configured on startup.
	 *
	 * This is helpful if attempting to connect to a serverless runner, so the serverless runner can define where the main endpoint lives.
	 *
	 * This is also helpful for setting up clean redirects as needed.
	 **/
	clientEndpoint?: string;
	/**
	 * Namespace that the client should use when connecting.
	 **/
	clientNamespace?: string;
	/**
	 * Token that the client should use when connecting.
	 **/
	clientToken?: string;
}

export function handleMetadataRequest(
	c: HonoContext,
	config: RegistryConfig,
	runnerKind: MetadataRunnerKind,
	clientEndpoint: string | undefined,
	clientNamespace: string | undefined,
	clientToken: string | undefined,
) {
	const response: MetadataResponse = {
		runtime: "rivetkit",
		version: VERSION,
		runner: {
			kind: runnerKind,
		},
		actorNames: buildActorNames(config),
		clientEndpoint,
		clientNamespace,
		clientToken,
	};

	return c.json(response);
}

export function handleHealthRequest(c: HonoContext) {
	return c.json({
		status: "ok",
		runtime: "rivetkit",
		version: VERSION,
	});
}
