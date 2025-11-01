import type { Next } from "hono";
import type { ContentfulStatusCode } from "hono/utils/http-status";
import * as errors from "@/actor/errors";
import { EXTRA_ERROR_LOG, getEnvUniversal, VERSION } from "@/utils";
import type { Logger } from "./log";

export function assertUnreachable(x: never): never {
	throw new Error(`Unreachable case: ${x}`);
}

/**
 * Safely stringifies an object, ensuring that the stringified object is under a certain size.
 * @param obj any object to stringify
 * @param maxSize maximum size of the stringified object in bytes
 * @returns stringified object
 */
export function safeStringify(obj: unknown, maxSize: number) {
	let size = 0;

	function replacer(key: string, value: unknown) {
		if (value === null || value === undefined) return value;
		const valueSize =
			typeof value === "string"
				? value.length
				: JSON.stringify(value).length;
		size += key.length + valueSize;

		if (size > maxSize) {
			throw new Error(
				`JSON object exceeds size limit of ${maxSize} bytes.`,
			);
		}

		return value;
	}

	return JSON.stringify(obj, replacer);
}

// TODO: Instead of doing this, use a temp var for state and attempt to write
// it. Roll back state if fails to serialize.

/**
 * Check if a value is CBOR serializable.
 * Optionally pass an onInvalid callback to receive the path to invalid values.
 *
 * For a complete list of supported CBOR tags, see:
 * https://github.com/kriszyp/cbor-x/blob/cc1cf9df8ba72288c7842af1dd374d73e34cdbc1/README.md#list-of-supported-tags-for-decoding
 */
export function isCborSerializable(
	value: unknown,
	onInvalid?: (path: string) => void,
	currentPath = "",
): boolean {
	// Handle primitive types directly
	if (value === null || value === undefined) {
		return true;
	}

	if (typeof value === "number") {
		if (!Number.isFinite(value)) {
			onInvalid?.(currentPath);
			return false;
		}
		return true;
	}

	if (typeof value === "boolean" || typeof value === "string") {
		return true;
	}

	// Handle BigInt (CBOR tags 2 and 3)
	if (typeof value === "bigint") {
		return true;
	}

	// Handle Date objects (CBOR tags 0 and 1)
	if (value instanceof Date) {
		return true;
	}

	// Handle typed arrays (CBOR tags 64-82)
	if (
		value instanceof Uint8Array ||
		value instanceof Uint8ClampedArray ||
		value instanceof Uint16Array ||
		value instanceof Uint32Array ||
		value instanceof BigUint64Array ||
		value instanceof Int8Array ||
		value instanceof Int16Array ||
		value instanceof Int32Array ||
		value instanceof BigInt64Array ||
		value instanceof Float32Array ||
		value instanceof Float64Array
	) {
		return true;
	}

	// Handle Map (CBOR tag 259)
	if (value instanceof Map) {
		for (const [key, val] of value.entries()) {
			const keyPath = currentPath
				? `${currentPath}.key(${String(key)})`
				: `key(${String(key)})`;
			const valPath = currentPath
				? `${currentPath}.value(${String(key)})`
				: `value(${String(key)})`;
			if (
				!isCborSerializable(key, onInvalid, keyPath) ||
				!isCborSerializable(val, onInvalid, valPath)
			) {
				return false;
			}
		}
		return true;
	}

	// Handle Set (CBOR tag 258)
	if (value instanceof Set) {
		let index = 0;
		for (const item of value.values()) {
			const itemPath = currentPath
				? `${currentPath}.set[${index}]`
				: `set[${index}]`;
			if (!isCborSerializable(item, onInvalid, itemPath)) {
				return false;
			}
			index++;
		}
		return true;
	}

	// Handle RegExp (CBOR tag 27)
	if (value instanceof RegExp) {
		return true;
	}

	// Handle Error objects (CBOR tag 27)
	if (value instanceof Error) {
		return true;
	}

	// Handle arrays
	if (Array.isArray(value)) {
		for (let i = 0; i < value.length; i++) {
			const itemPath = currentPath ? `${currentPath}[${i}]` : `[${i}]`;
			if (!isCborSerializable(value[i], onInvalid, itemPath)) {
				return false;
			}
		}
		return true;
	}

	// Handle plain objects and records (CBOR tags 105, 51, 57344-57599)
	if (typeof value === "object") {
		// Allow plain objects and objects with prototypes (for records and named objects)
		const proto = Object.getPrototypeOf(value);
		if (proto !== null && proto !== Object.prototype) {
			// Check if it's a known serializable object type
			const protoConstructor = value.constructor;
			if (protoConstructor && typeof protoConstructor.name === "string") {
				// Allow objects with named constructors (records, named objects)
				// This includes user-defined classes and built-in objects
				// that CBOR can serialize with tag 27 or record tags
			}
		}

		// Check all properties recursively
		for (const key in value) {
			const propPath = currentPath ? `${currentPath}.${key}` : key;
			if (
				!isCborSerializable(
					value[key as keyof typeof value],
					onInvalid,
					propPath,
				)
			) {
				return false;
			}
		}
		return true;
	}

	// Not serializable
	onInvalid?.(currentPath);
	return false;
}

export interface DeconstructedError {
	__type: "ActorError";
	statusCode: ContentfulStatusCode;
	public: boolean;
	group: string;
	code: string;
	message: string;
	metadata?: unknown;
}

/** Deconstructs error in to components that are used to build responses. */
export function deconstructError(
	error: unknown,
	logger: Logger,
	extraLog: Record<string, unknown>,
	exposeInternalError = false,
): DeconstructedError {
	// Build response error information. Only return errors if flagged as public in order to prevent leaking internal behavior.
	//
	// We log the error here instead of after generating the code & message because we need to log the original error, not the masked internal error.
	let statusCode: ContentfulStatusCode;
	let public_: boolean;
	let group: string;
	let code: string;
	let message: string;
	let metadata: unknown;
	if (errors.ActorError.isActorError(error) && error.public) {
		// Check if error has statusCode (could be ActorError instance or DeconstructedError)
		statusCode = (
			"statusCode" in error && error.statusCode ? error.statusCode : 400
		) as ContentfulStatusCode;
		public_ = true;
		group = error.group;
		code = error.code;
		message = getErrorMessage(error);
		metadata = error.metadata;

		logger.info({
			msg: "public error",
			group,
			code,
			message,
			...EXTRA_ERROR_LOG,
			...extraLog,
		});
	} else if (exposeInternalError) {
		if (errors.ActorError.isActorError(error)) {
			statusCode = 500;
			public_ = false;
			group = error.group;
			code = error.code;
			message = getErrorMessage(error);
			metadata = error.metadata;

			logger.info({
				msg: "internal error",
				group,
				code,
				message,
				...EXTRA_ERROR_LOG,
				...extraLog,
			});
		} else {
			statusCode = 500;
			public_ = false;
			group = "internal";
			code = errors.INTERNAL_ERROR_CODE;
			message = getErrorMessage(error);

			logger.info({
				msg: "internal error",
				group,
				code,
				message,
				...EXTRA_ERROR_LOG,
				...extraLog,
			});
		}
	} else {
		statusCode = 500;
		public_ = false;
		group = "internal";
		code = errors.INTERNAL_ERROR_CODE;
		message = errors.INTERNAL_ERROR_DESCRIPTION;
		metadata = {
			//url: `https://hub.rivet.dev/projects/${actorMetadata.project.slug}/environments/${actorMetadata.environment.slug}/actors?actorId=${actorMetadata.actor.id}`,
		} satisfies errors.InternalErrorMetadata;

		logger.warn({
			msg: "internal error",
			error: getErrorMessage(error),
			stack: (error as Error)?.stack,
			...EXTRA_ERROR_LOG,
			...extraLog,
		});
	}

	return {
		__type: "ActorError",
		statusCode,
		public: public_,
		group,
		code,
		message,
		metadata,
	};
}

export function stringifyError(error: unknown): string {
	if (error instanceof Error) {
		if (
			typeof process !== "undefined" &&
			getEnvUniversal("_RIVETKIT_ERROR_STACK") === "1"
		) {
			return `${error.name}: ${error.message}${error.stack ? `\n${error.stack}` : ""}`;
		} else {
			return `${error.name}: ${error.message}`;
		}
	} else if (typeof error === "string") {
		return error;
	} else if (typeof error === "object" && error !== null) {
		try {
			return `${JSON.stringify(error)}`;
		} catch {
			return "[cannot stringify error]";
		}
	} else {
		return `Unknown error: ${getErrorMessage(error)}`;
	}
}

function getErrorMessage(err: unknown): string {
	if (
		err &&
		typeof err === "object" &&
		"message" in err &&
		typeof err.message === "string"
	) {
		return err.message;
	} else {
		return String(err);
	}
}

/** Generates a `Next` handler to pass to middleware in order to be able to call arbitrary middleware. */
export function noopNext(): Next {
	return async () => {};
}
