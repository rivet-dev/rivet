import type { ActorKey } from "@/mod";

export const EMPTY_KEY = "/";
export const KEY_SEPARATOR = "/";

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
		parts.push(currentPart + "\\");
	} else if (isEmptyStringMarker) {
		parts.push("");
	} else if (currentPart !== "" || parts.length > 0) {
		parts.push(currentPart);
	}

	return parts;
}
