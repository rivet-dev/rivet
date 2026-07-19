import { jsonStringifyCompat } from "rivetkit/utils";

export function formatValue(value: unknown, pretty = false): string {
	try {
		const formatted = jsonStringifyCompat(value, pretty ? 2 : undefined);
		if (typeof formatted === "string") {
			return formatted;
		}
	} catch {
		// Fall through to a stable render-safe description.
	}

	if (value === undefined) return "undefined";
	if (typeof value === "function") {
		return value.name ? `[Function ${value.name}]` : "[Function]";
	}
	if (typeof value === "symbol") return String(value);
	return "[Unserializable value]";
}
