import * as cbor from "cbor-x";
import invariant from "invariant";
import type { z } from "zod";
import type { Encoding } from "@/actor/protocol/serde";
import { assertUnreachable } from "@/common/utils";
import type { VersionedDataHandler } from "@/common/versioned-data";
import type { HttpResponseError } from "@/schemas/client-protocol/mod";
import { HTTP_RESPONSE_ERROR_VERSIONED } from "@/schemas/client-protocol/versioned";
import {
	type HttpResponseError as HttpResponseErrorJson,
	HttpResponseErrorSchema,
} from "@/schemas/client-protocol-zod/mod";
import {
	contentTypeForEncoding,
	deserializeWithEncoding,
	encodingIsBinary,
	serializeWithEncoding,
} from "@/serde";
import { httpUserAgent } from "@/utils";
import { ActorError, HttpRequestError } from "./errors";
import { logger } from "./log";

export type WebSocketMessage = string | Blob | ArrayBuffer | Uint8Array;

export function messageLength(message: WebSocketMessage): number {
	if (message instanceof Blob) {
		return message.size;
	}
	if (message instanceof ArrayBuffer) {
		return message.byteLength;
	}
	if (message instanceof Uint8Array) {
		return message.byteLength;
	}
	if (typeof message === "string") {
		return message.length;
	}
	assertUnreachable(message);
}

export interface HttpRequestOpts<
	RequestBare,
	ResponseBare,
	RequestJson = RequestBare,
	ResponseJson = ResponseBare,
	Request = RequestBare,
	Response = ResponseBare,
> {
	method: string;
	url: string;
	headers: Record<string, string>;
	body?: Request;
	encoding: Encoding;
	skipParseResponse?: boolean;
	signal?: AbortSignal;
	customFetch?: (req: globalThis.Request) => Promise<globalThis.Response>;
	requestVersionedDataHandler: VersionedDataHandler<RequestBare> | undefined;
	responseVersionedDataHandler:
		| VersionedDataHandler<ResponseBare>
		| undefined;
	requestZodSchema: z.ZodType<RequestJson>;
	responseZodSchema: z.ZodType<ResponseJson>;
	requestToJson: (value: Request) => RequestJson;
	requestToBare: (value: Request) => RequestBare;
	responseFromJson: (value: ResponseJson) => Response;
	responseFromBare: (value: ResponseBare) => Response;
}

export async function sendHttpRequest<
	RequestBare = unknown,
	ResponseBare = unknown,
	RequestJson = RequestBare,
	ResponseJson = ResponseBare,
	Request = RequestBare,
	Response = ResponseBare,
>(
	opts: HttpRequestOpts<
		RequestBare,
		ResponseBare,
		RequestJson,
		ResponseJson,
		Request,
		Response
	>,
): Promise<Response> {
	logger().debug({
		msg: "sending http request",
		url: opts.url,
		encoding: opts.encoding,
	});

	// Serialize body
	let contentType: string | undefined;
	let bodyData: string | Uint8Array | undefined;
	if (opts.method === "POST" || opts.method === "PUT") {
		invariant(opts.body !== undefined, "missing body");
		contentType = contentTypeForEncoding(opts.encoding);
		bodyData = serializeWithEncoding<RequestBare, RequestJson, Request>(
			opts.encoding,
			opts.body,
			opts.requestVersionedDataHandler,
			opts.requestZodSchema,
			opts.requestToJson,
			opts.requestToBare,
		);
	}

	// Send request
	let response: globalThis.Response;
	try {
		// Make the HTTP request
		response = await (opts.customFetch ?? fetch)(
			new globalThis.Request(opts.url, {
				method: opts.method,
				headers: {
					...opts.headers,
					...(contentType
						? {
								"Content-Type": contentType,
							}
						: {}),
					"User-Agent": httpUserAgent(),
				},
				body: bodyData,
				credentials: "include",
				signal: opts.signal,
			}),
		);
	} catch (error) {
		throw new HttpRequestError(`Request failed: ${error}`, {
			cause: error,
		});
	}

	// Parse response error
	if (!response.ok) {
		// Attempt to parse structured data
		const bufferResponse = await response.arrayBuffer();
		let responseData: {
			group: string;
			code: string;
			message: string;
			metadata: unknown;
		};
		try {
			responseData = deserializeWithEncoding(
				opts.encoding,
				new Uint8Array(bufferResponse),
				HTTP_RESPONSE_ERROR_VERSIONED,
				HttpResponseErrorSchema,
				// JSON: metadata is already unknown
				(json): HttpResponseErrorJson => json as HttpResponseErrorJson,
				// BARE: decode ArrayBuffer metadata to unknown
				(bare): any => ({
					group: bare.group,
					code: bare.code,
					message: bare.message,
					metadata: bare.metadata
						? cbor.decode(new Uint8Array(bare.metadata))
						: null,
				}),
			);
		} catch (error) {
			//logger().warn("failed to cleanly parse error, this is likely because a non-structured response is being served", {
			//	error: stringifyError(error),
			//});

			// Error is not structured
			const textResponse = new TextDecoder("utf-8", {
				fatal: false,
			}).decode(bufferResponse);

			const rayId = response.headers.get("x-rivet-ray-id");

			if (rayId) {
				throw new HttpRequestError(
					`${response.statusText} (${response.status}) (Ray ID: ${rayId}):\n${textResponse}`,
				);
			} else {
				throw new HttpRequestError(
					`${response.statusText} (${response.status}):\n${textResponse}`,
				);
			}
		}

		// Throw structured error
		throw new ActorError(
			responseData.group,
			responseData.code,
			responseData.message,
			responseData.metadata,
		);
	}

	// Some requests don't need the success response to be parsed, so this can speed things up
	if (opts.skipParseResponse) {
		return undefined as Response;
	}

	// Parse the response based on encoding
	try {
		const buffer = new Uint8Array(await response.arrayBuffer());
		return deserializeWithEncoding<ResponseBare, ResponseJson, Response>(
			opts.encoding,
			buffer,
			opts.responseVersionedDataHandler,
			opts.responseZodSchema,
			opts.responseFromJson,
			opts.responseFromBare,
		);
	} catch (error) {
		throw new HttpRequestError(`Failed to parse response: ${error}`, {
			cause: error,
		});
	}
}
