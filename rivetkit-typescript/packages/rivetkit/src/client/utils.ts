import * as cbor from "cbor-x";
import invariant from "invariant";
import type { z } from "zod";
import { HttpResponseErrorSchema } from "@/actor/client-protocol-schema-json/mod";
import type { Encoding } from "@/actor/protocol/serde";
import { assertUnreachable } from "@/common/utils";
import type { VersionedDataHandler } from "@/common/versioned-data";
import type { HttpResponseError } from "@/schemas/client-protocol/mod";
import { HTTP_RESPONSE_ERROR_VERSIONED } from "@/schemas/client-protocol/versioned";
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
	RequestBody,
	ResponseBody,
	RequestJson = RequestBody,
	ResponseJson = ResponseBody,
> {
	method: string;
	url: string;
	headers: Record<string, string>;
	body?: RequestBody | RequestJson;
	encoding: Encoding;
	skipParseResponse?: boolean;
	signal?: AbortSignal;
	customFetch?: (req: Request) => Promise<Response>;
	requestVersionedDataHandler: VersionedDataHandler<RequestBody> | undefined;
	responseVersionedDataHandler:
		| VersionedDataHandler<ResponseBody>
		| undefined;
	requestZodSchema: z.ZodType<RequestJson>;
	responseZodSchema: z.ZodType<ResponseJson>;
}

export async function sendHttpRequest<
	RequestBody = unknown,
	ResponseBody = unknown,
	RequestJson = RequestBody,
	ResponseJson = ResponseBody,
>(
	opts: HttpRequestOpts<RequestBody, ResponseBody, RequestJson, ResponseJson>,
): Promise<ResponseBody | ResponseJson> {
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
		bodyData = serializeWithEncoding<RequestBody, RequestJson>(
			opts.encoding,
			opts.body,
			opts.requestVersionedDataHandler,
			opts.requestZodSchema,
		);
	}

	// Send request
	let response: Response;
	try {
		// Make the HTTP request
		response = await (opts.customFetch ?? fetch)(
			new Request(opts.url, {
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
		let responseData: HttpResponseError;
		try {
			responseData = deserializeWithEncoding(
				opts.encoding,
				new Uint8Array(bufferResponse),
				HTTP_RESPONSE_ERROR_VERSIONED,
				HttpResponseErrorSchema,
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

		// Decode metadata based on encoding - only binary encodings have CBOR-encoded metadata
		let decodedMetadata: unknown;
		if (responseData.metadata && encodingIsBinary(opts.encoding)) {
			decodedMetadata = cbor.decode(
				new Uint8Array(responseData.metadata),
			);
		}

		// Throw structured error
		throw new ActorError(
			responseData.group,
			responseData.code,
			responseData.message,
			decodedMetadata,
		);
	}

	// Some requests don't need the success response to be parsed, so this can speed things up
	if (opts.skipParseResponse) {
		return undefined as ResponseBody | ResponseJson;
	}

	// Parse the response based on encoding
	try {
		const buffer = new Uint8Array(await response.arrayBuffer());
		return deserializeWithEncoding<ResponseBody, ResponseJson>(
			opts.encoding,
			buffer,
			opts.responseVersionedDataHandler,
			opts.responseZodSchema,
		);
	} catch (error) {
		throw new HttpRequestError(`Failed to parse response: ${error}`, {
			cause: error,
		});
	}
}
