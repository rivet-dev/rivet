import * as cbor from "cbor-x";
import { z } from "zod/v4";
import type { AnyDatabaseProvider } from "@/actor/database";
import * as errors from "@/actor/errors";
import {
	CachedSerializer,
	type Encoding,
	type InputData,
} from "@/actor/protocol/serde";
import { deconstructError } from "@/common/utils";
import type * as protocol from "@/schemas/client-protocol/mod";
import {
	CURRENT_VERSION as CLIENT_PROTOCOL_CURRENT_VERSION,
	TO_CLIENT_VERSIONED,
	TO_SERVER_VERSIONED,
} from "@/schemas/client-protocol/versioned";
import {
	type ToClient as ToClientJson,
	ToClientSchema,
	type ToServer as ToServerJson,
	ToServerSchema,
} from "@/schemas/client-protocol-zod/mod";
import { deserializeWithEncoding } from "@/serde";
import {
	assertUnreachable,
	bufferToArrayBuffer,
	getEnvUniversal,
} from "../../utils";
import { CONN_SEND_MESSAGE_SYMBOL, type Conn } from "../conn/mod";
import { ActionContext } from "../contexts";
import type { ActorInstance } from "../instance/mod";
import type { EventSchemaConfig, QueueSchemaConfig } from "../schema";

interface MessageEventOpts {
	encoding: Encoding;
	maxIncomingMessageSize: number;
}

export function getValueLength(value: InputData): number {
	if (typeof value === "string") {
		return value.length;
	} else if (value instanceof Blob) {
		return value.size;
	} else if (
		value instanceof ArrayBuffer ||
		value instanceof SharedArrayBuffer ||
		value instanceof Uint8Array
	) {
		return value.byteLength;
	} else {
		assertUnreachable(value);
	}
}

export async function inputDataToBuffer(
	data: InputData,
): Promise<Uint8Array | string> {
	if (typeof data === "string") {
		return data;
	} else if (data instanceof Blob) {
		const arrayBuffer = await data.arrayBuffer();
		return new Uint8Array(arrayBuffer);
	} else if (data instanceof Uint8Array) {
		return data;
	} else if (
		data instanceof ArrayBuffer ||
		data instanceof SharedArrayBuffer
	) {
		return new Uint8Array(data);
	} else {
		throw new errors.MalformedMessage();
	}
}

export async function parseMessage(
	value: InputData,
	opts: MessageEventOpts,
): Promise<{
	body:
		| {
				tag: "ActionRequest";
				val: { id: bigint; name: string; args: unknown };
		  }
		| {
				tag: "SubscriptionRequest";
				val: { eventName: string; subscribe: boolean };
		  };
}> {
	// Validate value length
	const length = getValueLength(value);
	if (length > opts.maxIncomingMessageSize) {
		throw new errors.IncomingMessageTooLong();
	}

	// Convert value
	let buffer = await inputDataToBuffer(value);

	// HACK: For some reason, the output buffer needs to be cloned when using BARE encoding
	//
	// THis is likely because the input data is of type `Buffer` and there is an inconsistency in implementation that I am not aware of
	if (buffer instanceof Buffer) {
		buffer = new Uint8Array(buffer);
	}

	// Deserialize message
	return deserializeWithEncoding(
		opts.encoding,
		buffer,
		TO_SERVER_VERSIONED,
		ToServerSchema,
		// JSON: values are already the correct type
		(json: ToServerJson): any => json,
		// BARE: need to decode ArrayBuffer fields back to unknown
		(bare: protocol.ToServer): any => {
			if (bare.body.tag === "ActionRequest") {
				return {
					body: {
						tag: "ActionRequest",
						val: {
							id: bare.body.val.id,
							name: bare.body.val.name,
							args: cbor.decode(
								new Uint8Array(bare.body.val.args),
							),
						},
					},
				};
			} else {
				// SubscriptionRequest has no ArrayBuffer fields
				return bare;
			}
		},
	);
}

export interface ProcessMessageHandler<
	S,
	CP,
	CS,
	V,
	I,
	DB extends AnyDatabaseProvider,
	E extends EventSchemaConfig,
	Q extends QueueSchemaConfig,
> {
	onExecuteAction?: (
		ctx: ActionContext<S, CP, CS, V, I, DB, E, Q>,
		name: string,
		args: unknown[],
	) => Promise<unknown>;
	onSubscribe?: (
		eventName: string,
		conn: Conn<S, CP, CS, V, I, DB, E, Q>,
	) => Promise<void>;
	onUnsubscribe?: (
		eventName: string,
		conn: Conn<S, CP, CS, V, I, DB, E, Q>,
	) => Promise<void>;
}

export async function processMessage<
	S,
	CP,
	CS,
	V,
	I,
	DB extends AnyDatabaseProvider,
	E extends EventSchemaConfig,
	Q extends QueueSchemaConfig,
>(
	message: {
		body:
			| {
					tag: "ActionRequest";
					val: { id: bigint; name: string; args: unknown };
			  }
			| {
					tag: "SubscriptionRequest";
					val: { eventName: string; subscribe: boolean };
			  };
	},
	actor: ActorInstance<S, CP, CS, V, I, DB, E, Q>,
	conn: Conn<S, CP, CS, V, I, DB, E, Q>,
	handler: ProcessMessageHandler<S, CP, CS, V, I, DB, E, Q>,
) {
	let actionId: bigint | undefined;
	let actionName: string | undefined;

	try {
		if (message.body.tag === "ActionRequest") {
			// Action request

			if (handler.onExecuteAction === undefined) {
				throw new errors.Unsupported("Action");
			}

			const { id, name, args } = message.body.val;
			actionId = id;
			actionName = name;

			actor.rLog.debug({
				msg: "processing action request",
				actionId: id,
				actionName: name,
			});

			const ctx = new ActionContext<S, CP, CS, V, I, DB, E, Q>(
				actor,
				conn,
			);

			// Process the action request and wait for the result
			// This will wait for async actions to complete
			const output = await handler.onExecuteAction(
				ctx,
				name,
				args as unknown[],
			);

			actor.rLog.debug({
				msg: "sending action response",
				actionId: id,
				actionName: name,
				outputType: typeof output,
				isPromise: output instanceof Promise,
			});

			// Send the response back to the client
			conn[CONN_SEND_MESSAGE_SYMBOL](
				new CachedSerializer(
					output,
					TO_CLIENT_VERSIONED,
					CLIENT_PROTOCOL_CURRENT_VERSION,
					ToClientSchema,
					// JSON: output is the raw value
					(value): ToClientJson => ({
						body: {
							tag: "ActionResponse" as const,
							val: {
								id: id,
								output: value,
							},
						},
					}),
					// BARE/CBOR: output needs to be CBOR-encoded to ArrayBuffer
					(value): protocol.ToClient => ({
						body: {
							tag: "ActionResponse" as const,
							val: {
								id: id,
								output: bufferToArrayBuffer(cbor.encode(value)),
							},
						},
					}),
				),
			);

			actor.rLog.debug({ msg: "action response sent", id, name: name });
		} else if (message.body.tag === "SubscriptionRequest") {
			// Subscription request

			if (
				handler.onSubscribe === undefined ||
				handler.onUnsubscribe === undefined
			) {
				throw new errors.Unsupported("Subscriptions");
			}

			const { eventName, subscribe } = message.body.val;
			actor.rLog.debug({
				msg: "processing subscription request",
				eventName,
				subscribe,
			});

				if (subscribe) {
					await actor.assertCanSubscribe(
						new ActionContext<S, CP, CS, V, I, DB, E, Q>(
							actor,
							conn,
						),
						eventName,
					);
					await handler.onSubscribe(eventName, conn);
				} else {
					await handler.onUnsubscribe(eventName, conn);
				}

			actor.rLog.debug({
				msg: "subscription request completed",
				eventName,
				subscribe,
			});
		} else {
			assertUnreachable(message.body);
		}
	} catch (error) {
		const { group, code, message, metadata } = deconstructError(
			error,
			actor.rLog,
			{
				connectionId: conn.id,
				actionId,
				actionName,
			},
			getEnvUniversal("RIVET_EXPOSE_ERRORS") === "1" ||
				getEnvUniversal("NODE_ENV") === "development",
		);

		actor.rLog.debug({
			msg: "sending error response",
			actionId,
			actionName,
			code,
			message,
		});

		// Build response
		const errorData = { group, code, message, metadata, actionId };
		conn[CONN_SEND_MESSAGE_SYMBOL](
			new CachedSerializer(
				errorData,
				TO_CLIENT_VERSIONED,
				CLIENT_PROTOCOL_CURRENT_VERSION,
				ToClientSchema,
				// JSON: metadata is the raw value (keep as undefined if not present)
				(value): ToClientJson => {
					const val: any = {
						group: value.group,
						code: value.code,
						message: value.message,
						actionId:
							value.actionId !== undefined
								? value.actionId
								: null,
					};
					if (value.metadata !== undefined) {
						val.metadata = value.metadata;
					}
					return {
						body: {
							tag: "Error" as const,
							val,
						},
					};
				},
				// BARE/CBOR: metadata needs to be CBOR-encoded to ArrayBuffer
				// Note: protocol.Error expects `| null` for optional fields (BARE protocol)
				(value): protocol.ToClient => ({
					body: {
						tag: "Error" as const,
						val: {
							group: value.group,
							code: value.code,
							message: value.message,
							metadata: value.metadata
								? bufferToArrayBuffer(
										cbor.encode(value.metadata),
									)
								: null,
							actionId:
								value.actionId !== undefined
									? value.actionId
									: null,
						},
					},
				}),
			),
		);

		actor.rLog.debug({ msg: "error response sent", actionId, actionName });
	}
}

///**
// * Use `CachedSerializer` if serializing the same data repeatedly.
// */
//export function serialize<T>(value: T, encoding: Encoding): OutputData {
//	if (encoding === "json") {
//		return JSON.stringify(value);
//	} else if (encoding === "cbor") {
//		// TODO: Remove this hack, but cbor-x can't handle anything extra in data structures
//		const cleanValue = JSON.parse(JSON.stringify(value));
//		return cbor.encode(cleanValue);
//	} else {
//		assertUnreachable(encoding);
//	}
//}
//
//export async function deserialize(data: InputData, encoding: Encoding) {
//	if (encoding === "json") {
//		if (typeof data !== "string") {
//			actor.rLog.warn("received non-string for json parse");
//			throw new errors.MalformedMessage();
//		} else {
//			return JSON.parse(data);
//		}
//	} else if (encoding === "cbor") {
//		if (data instanceof Blob) {
//			const arrayBuffer = await data.arrayBuffer();
//			return cbor.decode(new Uint8Array(arrayBuffer));
//		} else if (data instanceof Uint8Array) {
//			return cbor.decode(data);
//		} else if (
//			data instanceof ArrayBuffer ||
//			data instanceof SharedArrayBuffer
//		) {
//			return cbor.decode(new Uint8Array(data));
//		} else {
//			actor.rLog.warn("received non-binary type for cbor parse");
//			throw new errors.MalformedMessage();
//		}
//	} else {
//		assertUnreachable(encoding);
//	}
//}
