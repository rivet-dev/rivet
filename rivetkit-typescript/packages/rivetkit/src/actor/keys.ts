import type { ActorKey } from "@/mod";

export const EMPTY_KEY = "/";
export const KEY_SEPARATOR = "/";
export const KEYS = {
	PERSIST_DATA: Uint8Array.from([1]),
	CONN_PREFIX: Uint8Array.from([2]),
	INSPECTOR_TOKEN: Uint8Array.from([3]),
	KV: Uint8Array.from([4]),
	QUEUE_PREFIX: Uint8Array.from([5]),
	LAST_PUSHED_ALARM: Uint8Array.from([6]),
	WORKFLOW_PREFIX: Uint8Array.from([6]),
	TRACES_PREFIX: Uint8Array.from([7]),
};

export const STORAGE_VERSION = {
	QUEUE: 1,
	WORKFLOW: 1,
	TRACES: 1,
} as const;

const STORAGE_VERSION_BYTES = {
	QUEUE: Uint8Array.from([STORAGE_VERSION.QUEUE]),
	WORKFLOW: Uint8Array.from([STORAGE_VERSION.WORKFLOW]),
	TRACES: Uint8Array.from([STORAGE_VERSION.TRACES]),
} as const;

const QUEUE_NAMESPACE = {
	METADATA: Uint8Array.from([1]),
	MESSAGES: Uint8Array.from([2]),
} as const;

const QUEUE_ID_BYTES = 8;

function concatPrefix(prefix: Uint8Array, suffix: Uint8Array): Uint8Array {
	const merged = new Uint8Array(prefix.length + suffix.length);
	merged.set(prefix, 0);
	merged.set(suffix, prefix.length);
	return merged;
}

const QUEUE_STORAGE_PREFIX = concatPrefix(
	KEYS.QUEUE_PREFIX,
	STORAGE_VERSION_BYTES.QUEUE,
);
const QUEUE_METADATA_KEY = concatPrefix(
	QUEUE_STORAGE_PREFIX,
	QUEUE_NAMESPACE.METADATA,
);
const QUEUE_MESSAGES_PREFIX = concatPrefix(
	QUEUE_STORAGE_PREFIX,
	QUEUE_NAMESPACE.MESSAGES,
);
const WORKFLOW_STORAGE_PREFIX = concatPrefix(
	KEYS.WORKFLOW_PREFIX,
	STORAGE_VERSION_BYTES.WORKFLOW,
);
const TRACES_STORAGE_PREFIX = concatPrefix(
	KEYS.TRACES_PREFIX,
	STORAGE_VERSION_BYTES.TRACES,
);

// The slash key form the actor REST API and the actor runtime use. The engine translates it to
// the canonical form (`encodeActorKey`) at those boundaries.
export function serializeActorKey(key: ActorKey): string {
	// Use a special marker for empty key arrays
	if (key.length === 0) {
		return EMPTY_KEY;
	}

	// Escape each key part to handle the separator and the empty key marker
	const escapedParts = key.map((part) => {
		// Handle empty strings by using a special marker
		if (part === "") {
			return "\\0"; // Use \0 as a marker for empty strings
		}

		// Escape backslashes first to avoid conflicts with our markers
		let escaped = part.replace(/\\/g, "\\\\");

		// Then escape separators
		escaped = escaped.replace(/\//g, `\\${KEY_SEPARATOR}`);

		return escaped;
	});

	return escapedParts.join(KEY_SEPARATOR);
}

export function deserializeActorKey(keyString: string | undefined): ActorKey {
	// Check for special empty key marker
	if (
		keyString === undefined ||
		keyString === null ||
		keyString === EMPTY_KEY
	) {
		return [];
	}

	// Split by unescaped separators and unescape the escaped characters
	const parts: string[] = [];
	let currentPart = "";
	let escaping = false;
	let isEmptyStringMarker = false;

	for (let i = 0; i < keyString.length; i++) {
		const char = keyString[i];

		if (escaping) {
			// Handle special escape sequences
			if (char === "0") {
				// \0 represents an empty string marker
				isEmptyStringMarker = true;
			} else {
				// This is an escaped character, add it directly
				currentPart += char;
			}
			escaping = false;
		} else if (char === "\\") {
			// Start of an escape sequence
			escaping = true;
		} else if (char === KEY_SEPARATOR) {
			// This is a separator
			if (isEmptyStringMarker) {
				parts.push("");
				isEmptyStringMarker = false;
			} else {
				parts.push(currentPart);
			}
			currentPart = "";
		} else {
			// Regular character
			currentPart += char;
		}
	}

	// Add the last part
	if (escaping) {
		// Incomplete escape at the end - treat as literal backslash
		parts.push(`${currentPart}\\`);
	} else if (isEmptyStringMarker) {
		parts.push("");
	} else if (currentPart !== "" || parts.length > 0) {
		parts.push(currentPart);
	}

	return parts;
}

// The canonical actor key: parts joined with `,`, with `\` and `,` escaped by a backslash, an
// empty part written as `\0`, and the empty key as the empty string. The engine hashes, stores
// and compares only canonical keys, and the gateway's `rvt-key` carries one. The Rust twin is
// `rivetkit_shared_types::actor_key`; both run the vectors in
// `rivetkit-rust/packages/shared-types/tests/fixtures/actor_key_vectors.json`.
const CANONICAL_SEPARATOR = ",";
const CANONICAL_EMPTY_PART = "\\0";

/** Encodes `key` in the canonical form. */
export function encodeActorKey(key: ActorKey): string {
	return key
		.map((part) => {
			if (part === "") {
				return CANONICAL_EMPTY_PART;
			}
			return part.replace(/\\/g, "\\\\").replace(/,/g, "\\,");
		})
		.join(CANONICAL_SEPARATOR);
}

/**
 * Decodes a canonical key, or an unescaped `rvt-key` value from an earlier client. Only `\\` and
 * `\,` are escapes; any other backslash is literal, and a part written as exactly `\0` is empty.
 */
export function decodeActorKey(key: string): ActorKey {
	if (key === "") {
		return [];
	}

	const parts: string[] = [];
	let raw = "";
	let current = "";
	const finishPart = () => {
		parts.push(raw === CANONICAL_EMPTY_PART ? "" : current);
		raw = "";
		current = "";
	};
	for (let i = 0; i < key.length; i++) {
		const char = key[i];
		if (char === CANONICAL_SEPARATOR) {
			finishPart();
			continue;
		}

		raw += char;
		if (char !== "\\") {
			current += char;
			continue;
		}

		const next = key[i + 1];
		if (next === "\\" || next === CANONICAL_SEPARATOR) {
			raw += next;
			current += next;
			i++;
		} else {
			current += char;
		}
	}
	finishPart();
	return parts;
}

/**
 * The gateway's `rvt-key` query value for `key`: the canonical key, or `undefined` when the
 * parameter is omitted, as it is for the empty key.
 */
export function actorKeyToGatewayQuery(key: ActorKey): string | undefined {
	if (key.length === 0) {
		return undefined;
	}
	return encodeActorKey(key);
}

/** Reads the gateway's `rvt-key` query value. A missing value is the empty key. */
export function actorKeyFromGatewayQuery(
	rvtKey: string | null | undefined,
): ActorKey {
	if (rvtKey === undefined || rvtKey === null) {
		return [];
	}
	return decodeActorKey(rvtKey);
}

export function makePrefixedKey(key: Uint8Array): Uint8Array {
	const prefixed = new Uint8Array(KEYS.KV.length + key.length);
	prefixed.set(KEYS.KV, 0);
	prefixed.set(key, KEYS.KV.length);
	return prefixed;
}

export function removePrefixFromKey(prefixedKey: Uint8Array): Uint8Array {
	return prefixedKey.slice(KEYS.KV.length);
}

export function makeWorkflowKey(key: Uint8Array): Uint8Array {
	return concatPrefix(WORKFLOW_STORAGE_PREFIX, key);
}

export function makeTracesKey(key: Uint8Array): Uint8Array {
	return concatPrefix(TRACES_STORAGE_PREFIX, key);
}

export function workflowStoragePrefix(): Uint8Array {
	return Uint8Array.from(WORKFLOW_STORAGE_PREFIX);
}

export function tracesStoragePrefix(): Uint8Array {
	return Uint8Array.from(TRACES_STORAGE_PREFIX);
}

export function queueStoragePrefix(): Uint8Array {
	return Uint8Array.from(QUEUE_STORAGE_PREFIX);
}

export function queueMetadataKey(): Uint8Array {
	return Uint8Array.from(QUEUE_METADATA_KEY);
}

export function queueMessagesPrefix(): Uint8Array {
	return Uint8Array.from(QUEUE_MESSAGES_PREFIX);
}

export function makeConnKey(connId: string): Uint8Array {
	const encoder = new TextEncoder();
	const connIdBytes = encoder.encode(connId);
	const key = new Uint8Array(KEYS.CONN_PREFIX.length + connIdBytes.length);
	key.set(KEYS.CONN_PREFIX, 0);
	key.set(connIdBytes, KEYS.CONN_PREFIX.length);
	return key;
}

export function makeQueueMessageKey(id: bigint): Uint8Array {
	const key = new Uint8Array(QUEUE_MESSAGES_PREFIX.length + QUEUE_ID_BYTES);
	key.set(QUEUE_MESSAGES_PREFIX, 0);
	const view = new DataView(key.buffer, key.byteOffset, key.byteLength);
	view.setBigUint64(QUEUE_MESSAGES_PREFIX.length, id, false);
	return key;
}

export function decodeQueueMessageKey(key: Uint8Array): bigint {
	const offset = QUEUE_MESSAGES_PREFIX.length;
	if (key.length < offset + QUEUE_ID_BYTES) {
		throw new Error("Queue key is too short");
	}
	for (let i = 0; i < QUEUE_MESSAGES_PREFIX.length; i++) {
		if (key[i] !== QUEUE_MESSAGES_PREFIX[i]) {
			throw new Error("Queue key has invalid prefix");
		}
	}
	const view = new DataView(
		key.buffer,
		key.byteOffset + offset,
		QUEUE_ID_BYTES,
	);
	return view.getBigUint64(0, false);
}
