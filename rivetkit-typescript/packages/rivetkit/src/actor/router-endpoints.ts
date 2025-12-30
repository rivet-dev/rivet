import * as cbor from "cbor-x";
import type { Context as HonoContext, HonoRequest } from "hono";
import type { AnyConn } from "@/actor/conn/mod";
import { ActionContext } from "@/actor/contexts/action";
import * as errors from "@/actor/errors";
import type { AnyActorInstance } from "@/actor/instance/mod";
import { type Encoding, EncodingSchema } from "@/actor/protocol/serde";
import {
	HEADER_ACTOR_QUERY,
	HEADER_CONN_PARAMS,
	HEADER_ENCODING,
	WS_PROTOCOL_CONN_PARAMS,
	WS_PROTOCOL_ENCODING,
} from "@/common/actor-router-consts";
import { stringifyError } from "@/common/utils";
import type * as protocol from "@/schemas/client-protocol/mod";
import {
	CURRENT_VERSION as CLIENT_PROTOCOL_CURRENT_VERSION,
	HTTP_ACTION_REQUEST_VERSIONED,
	HTTP_ACTION_RESPONSE_VERSIONED,
} from "@/schemas/client-protocol/versioned";
import {
	type HttpActionRequest as HttpActionRequestJson,
	HttpActionRequestSchema,
	type HttpActionResponse as HttpActionResponseJson,
	HttpActionResponseSchema,
} from "@/schemas/client-protocol-zod/mod";
import {
	contentTypeForEncoding,
	deserializeWithEncoding,
	serializeWithEncoding,
} from "@/serde";
import { bufferToArrayBuffer } from "@/utils";
import { createHttpDriver } from "./conn/drivers/http";
import { createRawRequestDriver } from "./conn/drivers/raw-request";
import type { ActorDriver } from "./driver";
import { loggerWithoutContext } from "./log";
import { RunnerConfig } from "@/registry/config/runner";

export interface ActionOpts {
	req?: HonoRequest;
	params: unknown;
	actionName: string;
	actionArgs: unknown[];
	actorId: string;
}

export interface ActionOutput {
	output: unknown;
}

export interface ConnsMessageOpts {
	req?: HonoRequest;
	connId: string;
	message: protocol.ToServer;
	actorId: string;
}

export interface FetchOpts {
	request: Request;
	actorId: string;
}

/**
 * Creates an action handler
 */
export async function handleAction(
	c: HonoContext,
	runConfig: RunnerConfig,
	actorDriver: ActorDriver,
	actionName: string,
	actorId: string,
) {
	const encoding = getRequestEncoding(c.req);
	const parameters = getRequestConnParams(c.req);

	// Validate incoming request
	const arrayBuffer = await c.req.arrayBuffer();

	// Check message size
	if (arrayBuffer.byteLength > runConfig.maxIncomingMessageSize) {
		throw new errors.IncomingMessageTooLong();
	}

	const request = deserializeWithEncoding(
		encoding,
		new Uint8Array(arrayBuffer),
		HTTP_ACTION_REQUEST_VERSIONED,
		HttpActionRequestSchema,
		// JSON: args is already the decoded value (raw object/array)
		(json: HttpActionRequestJson) => json.args,
		// BARE/CBOR: args is ArrayBuffer that needs CBOR-decoding
		(bare: protocol.HttpActionRequest) =>
			cbor.decode(new Uint8Array(bare.args)),
	);
	const actionArgs = request;

	// Invoke the action
	let actor: AnyActorInstance | undefined;
	let conn: AnyConn | undefined;
	let output: unknown | undefined;
	try {
		actor = await actorDriver.loadActor(actorId);

		actor.rLog.debug({ msg: "handling action", actionName, encoding });

		// Create conn
		conn = await actor.connectionManager.prepareAndConnectConn(
			createHttpDriver(),
			parameters,
			c.req.raw,
			c.req.path,
			c.req.header(),
		);

		// Call action
		const ctx = new ActionContext(actor, conn!);
		output = await actor.executeAction(ctx, actionName, actionArgs);
	} finally {
		if (conn) {
			conn.disconnect();
		}
	}

	// Send response
	const serialized = serializeWithEncoding(
		encoding,
		output,
		HTTP_ACTION_RESPONSE_VERSIONED,
		CLIENT_PROTOCOL_CURRENT_VERSION,
		HttpActionResponseSchema,
		// JSON: output is the raw value (will be serialized by jsonStringifyCompat)
		(value): HttpActionResponseJson => ({ output: value }),
		// BARE/CBOR: output needs to be CBOR-encoded to ArrayBuffer
		(value): protocol.HttpActionResponse => ({
			output: bufferToArrayBuffer(cbor.encode(value)),
		}),
	);

	// Check outgoing message size
	const messageSize = serialized instanceof Uint8Array ? serialized.byteLength : serialized.length;
	if (messageSize > runConfig.maxOutgoingMessageSize) {
		throw new errors.OutgoingMessageTooLong();
	}

	// TODO: Remove any, Hono is being a dumbass
	return c.body(serialized as Uint8Array as any, 200, {
		"Content-Type": contentTypeForEncoding(encoding),
	});
}

export async function handleRawRequest(
	c: HonoContext,
	req: Request,
	actorDriver: ActorDriver,
	actorId: string,
): Promise<Response> {
	const actor = await actorDriver.loadActor(actorId);
	const parameters = getRequestConnParams(c.req);

	// Track connection outside of scope for cleanup
	let createdConn: AnyConn | undefined;

	try {
		const conn = await actor.connectionManager.prepareAndConnectConn(
			createRawRequestDriver(),
			parameters,
			req,
			c.req.path,
			c.req.header(),
		);

		createdConn = conn;

		return await actor.handleRawRequest(conn, req);
	} finally {
		// Clean up the connection after the request completes
		if (createdConn) {
			createdConn.disconnect();
		}
	}
}

// Helper to get the connection encoding from a request
//
// Defaults to JSON if not provided so we can support vanilla curl requests easily.
export function getRequestEncoding(req: HonoRequest): Encoding {
	const encodingParam = req.header(HEADER_ENCODING);
	if (!encodingParam) {
		return "json";
	}

	const result = EncodingSchema.safeParse(encodingParam);
	if (!result.success) {
		throw new errors.InvalidEncoding(encodingParam as string);
	}

	return result.data;
}

export function getRequestExposeInternalError(_req: Request): boolean {
	// Unipmlemented
	return false;
}

export function getRequestQuery(c: HonoContext): unknown {
	// Get query parameters for actor lookup
	const queryParam = c.req.header(HEADER_ACTOR_QUERY);
	if (!queryParam) {
		loggerWithoutContext().error({ msg: "missing query parameter" });
		throw new errors.InvalidRequest("missing query");
	}

	// Parse the query JSON and validate with schema
	try {
		const parsed = JSON.parse(queryParam);
		return parsed;
	} catch (error) {
		loggerWithoutContext().error({ msg: "invalid query json", error });
		throw new errors.InvalidQueryJSON(error);
	}
}

// Helper to get connection parameters for the request
export function getRequestConnParams(req: HonoRequest): unknown {
	const paramsParam = req.header(HEADER_CONN_PARAMS);
	if (!paramsParam) {
		return null;
	}

	try {
		return JSON.parse(paramsParam);
	} catch (err) {
		throw new errors.InvalidParams(
			`Invalid params JSON: ${stringifyError(err)}`,
		);
	}
}
