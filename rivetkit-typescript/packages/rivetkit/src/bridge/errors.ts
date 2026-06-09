import { isRivetErrorLike, RivetError } from "@/actor/errors";
import type { BridgeErrorPayload } from "./protocol";

/** Flatten any thrown value into a structured-clone-safe payload. */
export function toBridgeErrorPayload(error: unknown): BridgeErrorPayload {
	if (isRivetErrorLike(error)) {
		return {
			message: error.message,
			group: error.group,
			code: error.code,
			metadata: sanitizeMetadata(error.metadata),
			public: error.public,
			statusCode: error.statusCode,
			stack: error instanceof Error ? error.stack : undefined,
		};
	}
	if (error instanceof Error) {
		return { message: error.message, stack: error.stack };
	}
	return { message: String(error) };
}

/**
 * Rebuild a throwable error on the other side of the bridge. RivetError-style
 * payloads become real RivetError instances so downstream classification
 * (deconstructError, bridge encoding) treats them as structured errors.
 */
export function fromBridgeErrorPayload(payload: BridgeErrorPayload): Error {
	if (payload.group !== undefined && payload.code !== undefined) {
		const error = new RivetError(
			payload.group,
			payload.code,
			payload.message,
			{
				metadata: payload.metadata,
				public: payload.public,
				statusCode: payload.statusCode,
			},
		);
		if (payload.stack) {
			error.stack = payload.stack;
		}
		return error;
	}
	const error = new Error(payload.message);
	if (payload.stack) {
		error.stack = payload.stack;
	}
	return error;
}

/** Metadata must survive structured clone; drop anything that cannot. */
function sanitizeMetadata(metadata: unknown): unknown {
	if (metadata === undefined || metadata === null) {
		return metadata;
	}
	try {
		structuredClone(metadata);
		return metadata;
	} catch {
		return undefined;
	}
}
