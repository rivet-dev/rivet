/**
 * Shared helpers for serializing state across actor boundaries.
 *
 * Rivet actors persist JSON-serializable state. We base64-encode binary data
 * (stream chunks, encryption keys) so `Uint8Array` survives persistence.
 */

export function encodeBinary(data: string | Uint8Array): string {
	if (typeof data === "string") {
		return Buffer.from(data, "utf8").toString("base64");
	}
	return Buffer.from(data).toString("base64");
}

export function decodeBinary(encoded: string): Uint8Array {
	return new Uint8Array(Buffer.from(encoded, "base64"));
}

export function nowMs(): number {
	return Date.now();
}

export function toDate(value: number | string | Date | undefined): Date {
	if (value === undefined) return new Date();
	if (value instanceof Date) return value;
	return new Date(value);
}

export function serializeDates<T>(obj: T): T {
	if (obj === null || obj === undefined) return obj;
	if (obj instanceof Date) return obj.toISOString() as unknown as T;
	if (Array.isArray(obj)) {
		return obj.map((item) => serializeDates(item)) as unknown as T;
	}
	if (typeof obj === "object") {
		const result: Record<string, unknown> = {};
		for (const [key, value] of Object.entries(obj as object)) {
			result[key] = serializeDates(value);
		}
		return result as unknown as T;
	}
	return obj;
}

export function deserializeDates<T>(obj: T, dateFields: string[] = []): T {
	if (obj === null || obj === undefined) return obj;
	if (Array.isArray(obj)) {
		return obj.map((item) => deserializeDates(item, dateFields)) as unknown as T;
	}
	if (typeof obj === "object") {
		const result: Record<string, unknown> = {};
		for (const [key, value] of Object.entries(obj as object)) {
			if (
				dateFields.includes(key) &&
				(typeof value === "string" || typeof value === "number")
			) {
				result[key] = new Date(value as string | number);
			} else if (typeof value === "object") {
				result[key] = deserializeDates(value, dateFields);
			} else {
				result[key] = value;
			}
		}
		return result as unknown as T;
	}
	return obj;
}

export const RUN_DATE_FIELDS = [
	"createdAt",
	"updatedAt",
	"startedAt",
	"finishedAt",
	"disposedAt",
	"triggeredAt",
	"requestedAt",
];
