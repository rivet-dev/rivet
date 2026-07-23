import { z } from "zod/v4";
import type {
	QueueMessageStatus,
	QueueReceipt,
	QueueSendOptions,
	QueueSendReceipt,
} from "@/actor/config";
import {
	HEADER_CONN_PARAMS,
	HEADER_ENCODING,
} from "@/common/actor-router-consts";
import type * as protocol from "@/common/client-protocol";
import {
	CURRENT_VERSION as CLIENT_PROTOCOL_CURRENT_VERSION,
	HTTP_QUEUE_SEND_REQUEST_VERSIONED,
	HTTP_QUEUE_SEND_RESPONSE_VERSIONED,
	HTTP_QUEUE_STATUS_RESPONSE_VERSIONED,
} from "@/common/client-protocol-versioned";
import {
	type HttpQueueSendRequest as HttpQueueSendRequestJson,
	HttpQueueSendRequestSchema,
	type HttpQueueSendResponse as HttpQueueSendResponseJson,
	HttpQueueSendResponseSchema,
	type HttpQueueStatusResponse as HttpQueueStatusResponseJson,
	HttpQueueStatusResponseSchema,
} from "@/common/client-protocol-zod";
import type { Encoding, JsonCompatValue } from "@/common/encoding";
import { encodeCborCompat } from "@/serde";
import { bufferToArrayBuffer } from "@/utils";
import { sendHttpRequest } from "./utils";

export type {
	QueueMessageStatus,
	QueueReceipt,
	QueueSendOptions,
	QueueSendReceipt,
} from "@/actor/config";

export interface QueueSender {
	send(
		name: string,
		body: unknown,
		options?: QueueSendOptions,
	): Promise<QueueSendReceipt>;
	receipt(id: string): QueueReceipt;
}

interface QueueSenderOptions {
	encoding: Encoding;
	params: unknown;
	customFetch: (request: Request) => Promise<Response>;
}

export function createQueueSender(
	senderOptions: QueueSenderOptions,
): QueueSender {
	const headers = (): Record<string, string> => ({
		[HEADER_ENCODING]: senderOptions.encoding,
		...(senderOptions.params !== undefined
			? { [HEADER_CONN_PARAMS]: JSON.stringify(senderOptions.params) }
			: {}),
	});

	function receipt(id: string): QueueReceipt {
		return {
			id,
			status: (options) => getStatus(id, options?.signal),
		};
	}

	async function send(
		name: string,
		body: unknown,
		options?: QueueSendOptions,
	): Promise<QueueSendReceipt> {
		validateQueueSendDelay(options?.delay);
		const result = await sendHttpRequest<
			protocol.HttpQueueSendRequest,
			protocol.HttpQueueSendResponse,
			HttpQueueSendRequestJson,
			HttpQueueSendResponseJson,
			{
				body: unknown;
				name?: string;
				dedupeKey?: string;
				delay?: number;
			},
			{ receiptId: string; deduplicated: boolean }
		>({
			url: `http://actor/queue/${encodeURIComponent(name)}`,
			method: "POST",
			headers: headers(),
			body: {
				body,
				name,
				dedupeKey: options?.dedupeKey,
				delay: options?.delay,
			},
			encoding: senderOptions.encoding,
			customFetch: senderOptions.customFetch,
			signal: options?.signal,
			requestVersion: CLIENT_PROTOCOL_CURRENT_VERSION,
			requestVersionedDataHandler: HTTP_QUEUE_SEND_REQUEST_VERSIONED,
			responseVersion: CLIENT_PROTOCOL_CURRENT_VERSION,
			responseVersionedDataHandler: HTTP_QUEUE_SEND_RESPONSE_VERSIONED,
			requestZodSchema: HttpQueueSendRequestSchema,
			responseZodSchema: HttpQueueSendResponseSchema,
			requestToJson: (value) => value,
			requestToBare: (value) => ({
				name: value.name ?? name,
				body: bufferToArrayBuffer(
					encodeCborCompat(value.body as JsonCompatValue),
				),
				dedupeKey: value.dedupeKey ?? null,
				delay: value.delay === undefined ? null : BigInt(value.delay),
			}),
			responseFromJson: (value) => value,
			responseFromBare: (value) => value,
		});
		return {
			...receipt(result.receiptId),
			deduplicated: result.deduplicated,
		};
	}

	async function getStatus(
		id: string,
		signal?: AbortSignal,
	): Promise<QueueMessageStatus> {
		const status = await sendHttpRequest<
			never,
			protocol.HttpQueueStatusResponse,
			null,
			HttpQueueStatusResponseJson,
			null,
			HttpQueueStatusResponseJson
		>({
			url: `http://actor/queue/receipts/${encodeURIComponent(id)}`,
			method: "GET",
			headers: headers(),
			encoding: senderOptions.encoding,
			customFetch: senderOptions.customFetch,
			signal,
			requestVersion: undefined,
			requestVersionedDataHandler: undefined,
			responseVersion: CLIENT_PROTOCOL_CURRENT_VERSION,
			responseVersionedDataHandler: HTTP_QUEUE_STATUS_RESPONSE_VERSIONED,
			requestZodSchema: z.null(),
			responseZodSchema: HttpQueueStatusResponseSchema,
			requestToJson: (value) => value,
			requestToBare: () => {
				throw new Error("queue receipt status GET has no request body");
			},
			responseFromJson: (value) => value,
			responseFromBare: (value) =>
				({
					state: value.state,
					attempts:
						value.attempts === null ? undefined : Number(value.attempts),
					createdAt:
						value.createdAt === null ? undefined : Number(value.createdAt),
					availableAt:
						value.availableAt === null
							? undefined
							: Number(value.availableAt),
					startedAt:
						value.startedAt === null ? undefined : Number(value.startedAt),
					completedAt:
						value.completedAt === null
							? undefined
							: Number(value.completedAt),
					failedAt:
						value.failedAt === null ? undefined : Number(value.failedAt),
					consumedAt:
						value.consumedAt === null
							? undefined
							: Number(value.consumedAt),
				}) as HttpQueueStatusResponseJson,
		});
		return queueStatusFromWire(status);
	}

	return { send, receipt };
}

function validateQueueSendDelay(delay: number | undefined): void {
	if (
		delay !== undefined &&
		(!Number.isSafeInteger(delay) || delay < 0)
	) {
		throw new RangeError("Queue delay must be a non-negative safe integer");
	}
}

function queueStatusFromWire(
	status: HttpQueueStatusResponseJson,
): QueueMessageStatus {
	const date = (value: number) => new Date(value);
	switch (status.state) {
		case "queued":
			return {
				state: status.state,
				attempts: status.attempts,
				createdAt: date(status.createdAt),
			};
		case "delayed":
		case "retrying":
			return {
				state: status.state,
				attempts: status.attempts,
				createdAt: date(status.createdAt),
				availableAt: date(status.availableAt),
			};
		case "processing":
			return {
				state: status.state,
				attempts: status.attempts,
				createdAt: date(status.createdAt),
				startedAt: date(status.startedAt),
			};
		case "succeeded":
			return {
				state: status.state,
				attempts: status.attempts,
				createdAt: date(status.createdAt),
				completedAt: date(status.completedAt),
			};
		case "deadLettered":
			return {
				state: status.state,
				attempts: status.attempts,
				createdAt: date(status.createdAt),
				failedAt: date(status.failedAt),
			};
		case "consumed":
			return {
				state: status.state,
				createdAt: date(status.createdAt),
				consumedAt: date(status.consumedAt),
			};
		case "unknown":
			return { state: status.state };
	}
}
