import { decode } from "cbor-x";

/**
 * Decodes the bytes stored in `_rivet_actor_state.state` into the value
 * that `c.state` held.
 */
export function decodeLegacyState(bytes: Uint8Array): unknown {
  return revive(decode(bytes));
}

// Values CBOR cannot represent directly are stored as `[tag, data]` pairs
function revive(value: unknown): unknown {
  if (Array.isArray(value)) {
    const [tag, data] = value;
    if (value.length === 2 && typeof tag === "string" && tag.startsWith("$")) {
      if (tag === "$BigInt") return BigInt(data);
      if (tag === "$Undefined") return undefined;
      if (tag === "$Set") return new Set((data as unknown[]).map(revive));
      if (tag === "$Uint8Array") return base64ToBytes(data);
      if (tag === "$ArrayBuffer") return base64ToBytes(data).buffer;
      // A user array whose first item starts with `$` is escaped as `$$`
      if (tag.startsWith("$$")) return [tag.slice(1), revive(data)];
    }
    return value.map(revive);
  }

  if (value instanceof Map) {
    return new Map([...value].map(([k, v]) => [revive(k), revive(v)]));
  }

  if (value !== null && typeof value === "object" && value.constructor === Object) {
    return Object.fromEntries(
      Object.entries(value).map(([k, v]) => [k, revive(v)]),
    );
  }

  return value;
}

function base64ToBytes(base64: string): Uint8Array {
  return Uint8Array.from(atob(base64), (char) => char.charCodeAt(0));
}
