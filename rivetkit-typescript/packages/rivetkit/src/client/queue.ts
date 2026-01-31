import * as cbor from "cbor-x";
import type { Encoding } from "@/actor/protocol/serde";
import { HEADER_CONN_PARAMS, HEADER_ENCODING } from "@/driver-helpers/mod";
import type * as protocol from "@/schemas/client-protocol/mod";
import {
	CURRENT_VERSION as CLIENT_PROTOCOL_CURRENT_VERSION,
	HTTP_QUEUE_SEND_REQUEST_VERSIONED,
	HTTP_QUEUE_SEND_RESPONSE_VERSIONED,
} from "@/schemas/client-protocol/versioned";
import {
	type HttpQueueSendRequest as HttpQueueSendRequestJson,
	HttpQueueSendRequestSchema,
	type HttpQueueSendResponse as HttpQueueSendResponseJson,
	HttpQueueSendResponseSchema,
} from "@/schemas/client-protocol-zod/mod";
import { bufferToArrayBuffer } from "@/utils";
import { sendHttpRequest } from "./utils";

export interface QueueSender {
	send(
		name: string,
		body: unknown,
		options?: QueueSendOptions | AbortSignal,
	): Promise<QueueSendResult | void>;
}

export interface QueueNameSender {
	send(
		body: unknown,
		options?: QueueSendOptions | AbortSignal,
	): Promise<QueueSendResult | void>;
}

export type QueueProxy = QueueSender & {
	[key: string]: QueueNameSender;
};

export interface QueueSendOptions {
	wait?: boolean;
	timeout?: number;
	signal?: AbortSignal;
}

export interface QueueSendResult {
	status: "completed" | "timedOut";
	response?: unknown;
}

interface QueueSenderOptions {
	encoding: Encoding;
	params: unknown;
	customFetch: (request: Request) => Promise<Response>;
}

export function createQueueSender(options: QueueSenderOptions): QueueSender {
	return {
		async send(
			name: string,
			body: unknown,
			options?: QueueSendOptions | AbortSignal,
		): Promise<QueueSendResult | void> {
			const normalizedOptions =
				options instanceof AbortSignal ? { signal: options } : options;
			const wait = normalizedOptions?.wait ?? false;
			const timeout = normalizedOptions?.timeout;

			const result = await sendHttpRequest<
				protocol.HttpQueueSendRequest,
				protocol.HttpQueueSendResponse,
				HttpQueueSendRequestJson,
				HttpQueueSendResponseJson,
				{ body: unknown; wait?: boolean; timeout?: number; name?: string },
				QueueSendResult
			>({
				url: `http://actor/queue/${encodeURIComponent(name)}`,
				method: "POST",
				headers: {
					[HEADER_ENCODING]: options.encoding,
					...(options.params !== undefined
						? {
								[HEADER_CONN_PARAMS]: JSON.stringify(
									options.params,
								),
							}
						: {}),
				},
				body: { body, wait, timeout },
				encoding: options.encoding,
				customFetch: options.customFetch,
				signal: normalizedOptions?.signal,
				requestVersion: CLIENT_PROTOCOL_CURRENT_VERSION,
				requestVersionedDataHandler: HTTP_QUEUE_SEND_REQUEST_VERSIONED,
				responseVersion: CLIENT_PROTOCOL_CURRENT_VERSION,
				responseVersionedDataHandler:
					HTTP_QUEUE_SEND_RESPONSE_VERSIONED,
				requestZodSchema: HttpQueueSendRequestSchema,
				responseZodSchema: HttpQueueSendResponseSchema,
				requestToJson: (value): HttpQueueSendRequestJson => ({
					...value,
					name,
				}),
				requestToBare: (value): protocol.HttpQueueSendRequest => ({
					name: value.name ?? name,
					body: bufferToArrayBuffer(cbor.encode(value.body)),
					wait: value.wait ?? false,
					timeout: value.timeout ?? null,
				}),
				responseFromJson: (json): QueueSendResult => {
					if (json.response === undefined) {
						return { status: json.status };
					}
					return {
						status: json.status,
						response: json.response,
					};
				},
				responseFromBare: (bare): QueueSendResult => {
					if (bare.response === null || bare.response === undefined) {
						return { status: bare.status };
					}
					return {
						status: bare.status,
						response: cbor.decode(new Uint8Array(bare.response)),
					};
				},
			});

			if (wait) {
				return result;
			}
			return;
		},
	};
}

export function createQueueProxy(sender: QueueSender): QueueProxy {
	const methodCache = new Map<string, QueueNameSender>();
	return new Proxy(sender, {
		get(target, prop: string | symbol, receiver: unknown) {
			if (typeof prop === "symbol") {
				return Reflect.get(target, prop, receiver);
			}

			if (prop in target) {
				const value = Reflect.get(target, prop, target);
				if (typeof value === "function") {
					return value.bind(target);
				}
				return value;
			}

			if (prop === "then") return undefined;

			if (typeof prop === "string") {
				let method = methodCache.get(prop);
				if (!method) {
					method = {
						send: (
							body: unknown,
							options?: QueueSendOptions | AbortSignal,
						) => target.send(prop, body, options),
					};
					methodCache.set(prop, method);
				}
				return method;
			}
		},
		has(target, prop: string | symbol) {
			if (typeof prop === "string") {
				return true;
			}
			return Reflect.has(target, prop);
		},
	}) as QueueProxy;
}
