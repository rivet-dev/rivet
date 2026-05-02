// Decodes hex-encoded FoundationDB tuples.
// Supports the subset used by SimpleTupleValue in
// engine/packages/engine/src/util/udb.rs:
//   u64, i64, f64, uuid, Id (custom 0x40), string, bytes, nested.
// See https://github.com/apple/foundationdb/blob/main/design/tuple.md

const fs = require("node:fs");
const path = require("node:path");

const KEY_NAMES = loadKeyNames();

function loadKeyNames() {
	const keysPath = path.resolve(
		__dirname,
		"../../engine/packages/universaldb/src/utils/keys.rs",
	);
	const text = fs.readFileSync(keysPath, "utf8");
	const map = new Map();
	const re = /\(\s*(\d+)\s*,\s*[A-Z0-9_]+\s*,\s*"([^"]+)"\s*\)/g;
	let m;
	while ((m = re.exec(text)) !== null) {
		map.set(Number(m[1]), m[2]);
	}
	return map;
}

const hexInput = process.argv[2];
if (!hexInput) {
	console.error("Usage: node decode_fdb_tuple.js <hex>");
	process.exit(1);
}

const buf = Buffer.from(hexInput.replace(/^0x|^\/|\/$/, ""), "hex");

const CODE = {
	NIL: 0x00,
	BYTES: 0x01,
	STRING: 0x02,
	NESTED: 0x05,
	NEG_INT_START: 0x0c, // 8-byte negative
	INT_ZERO: 0x14,
	POS_INT_END: 0x1c, // 8-byte positive
	FLOAT: 0x20,
	DOUBLE: 0x21,
	UUID: 0x30,
	VERSIONSTAMP: 0x33,
	ID: 0x40,
};

function decodeBytesEscaped(input, offset) {
	const out = [];
	while (offset < input.length) {
		const b = input[offset];
		if (b === 0x00) {
			if (offset + 1 < input.length && input[offset + 1] === 0xff) {
				out.push(0x00);
				offset += 2;
			} else {
				return { bytes: Buffer.from(out), next: offset + 1 };
			}
		} else {
			out.push(b);
			offset += 1;
		}
	}
	return { bytes: Buffer.from(out), next: offset };
}

function decodeId(input, offset) {
	// 0x40 already consumed. Next byte is version, then payload.
	const version = input[offset];
	if (version !== 1) {
		throw new Error(`unsupported Id version: ${version}`);
	}
	const total = 19; // version + 18 payload
	const slice = input.slice(offset, offset + total);
	const str = encodeBase36Id(slice);
	return { value: str, next: offset + total };
}

function encodeBase36Id(bytes19) {
	// Mirrors Display impl in engine/packages/util-id/src/lib.rs.
	// Treats bytes19 as little-endian-from-index-0 big integer and emits 30
	// base36 chars (least-significant digit first).
	const temp = Array.from(bytes19);
	const out = [];
	for (let i = 0; i < 30; i++) {
		let rem = 0;
		for (let j = temp.length - 1; j >= 0; j--) {
			const v = (rem << 8) | temp[j];
			temp[j] = Math.floor(v / 36);
			rem = v % 36;
		}
		out.push(rem < 10 ? String.fromCharCode(0x30 + rem) : String.fromCharCode(0x61 + rem - 10));
	}
	return out.join("");
}

function decodeUuid(input, offset) {
	const b = input.slice(offset, offset + 16);
	const hex = b.toString("hex");
	const uuid = `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20, 32)}`;
	return { value: uuid, next: offset + 16 };
}

function decodeFloatLike(input, offset, size) {
	const bytes = Buffer.from(input.slice(offset, offset + size));
	// FDB float/double: if sign bit set, clear it; else flip all bits.
	if (bytes[0] & 0x80) {
		bytes[0] &= 0x7f;
	} else {
		for (let i = 0; i < bytes.length; i++) bytes[i] = ~bytes[i] & 0xff;
	}
	const value = size === 4 ? bytes.readFloatBE(0) : bytes.readDoubleBE(0);
	return { value, next: offset + size };
}

function decodeInt(input, offset, code) {
	if (code === CODE.INT_ZERO) {
		return { value: 0n, next: offset };
	}
	if (code > CODE.INT_ZERO && code <= CODE.POS_INT_END) {
		const n = code - CODE.INT_ZERO;
		let v = 0n;
		for (let i = 0; i < n; i++) v = (v << 8n) | BigInt(input[offset + i]);
		return { value: v, next: offset + n };
	}
	if (code >= CODE.NEG_INT_START && code < CODE.INT_ZERO) {
		const n = CODE.INT_ZERO - code;
		let v = 0n;
		for (let i = 0; i < n; i++) v = (v << 8n) | BigInt(input[offset + i]);
		// Stored as ~(-x), so reconstruct the negative magnitude.
		const max = (1n << BigInt(n * 8)) - 1n;
		v = -(max - v);
		return { value: v, next: offset + n };
	}
	throw new Error(`not an int code: 0x${code.toString(16)}`);
}

function decodeOne(input, offset, depth) {
	if (offset >= input.length) {
		throw new Error("unexpected end of input");
	}
	const code = input[offset];
	offset += 1;

	switch (code) {
		case CODE.NIL:
			// In nested tuples, NIL may be encoded as 0x00 0xff. The terminator
			// is handled by the nested decoder; a bare NIL means null.
			if (depth > 0 && input[offset] === 0xff) {
				return { value: null, next: offset + 1 };
			}
			return { value: null, next: offset };
		case CODE.BYTES: {
			const { bytes, next } = decodeBytesEscaped(input, offset);
			return { value: { type: "bytes", hex: bytes.toString("hex") }, next };
		}
		case CODE.STRING: {
			const { bytes, next } = decodeBytesEscaped(input, offset);
			return { value: bytes.toString("utf8"), next };
		}
		case CODE.NESTED: {
			const items = [];
			while (offset < input.length) {
				if (input[offset] === 0x00) {
					if (input[offset + 1] === 0xff) {
						items.push(null);
						offset += 2;
						continue;
					}
					offset += 1;
					return { value: items, next: offset };
				}
				const r = decodeOne(input, offset, depth + 1);
				items.push(r.value);
				offset = r.next;
			}
			return { value: items, next: offset };
		}
		case CODE.FLOAT:
			return decodeFloatLike(input, offset, 4);
		case CODE.DOUBLE:
			return decodeFloatLike(input, offset, 8);
		case CODE.UUID:
			return decodeUuid(input, offset);
		case CODE.VERSIONSTAMP: {
			// 96-bit versionstamp: 8-byte db version, 2-byte batch version, 2-byte user ordering.
			const dbVersion = input.readBigUInt64BE(offset);
			const batchVersion = input.readUInt16BE(offset + 8);
			const userVersion = input.readUInt16BE(offset + 10);
			return { value: { type: "versionstamp", dbVersion, batchVersion, userVersion }, next: offset + 12 };
		}
		case CODE.ID:
			return decodeId(input, offset);
		default:
			if (
				code === CODE.INT_ZERO ||
				(code > CODE.INT_ZERO && code <= CODE.POS_INT_END) ||
				(code >= CODE.NEG_INT_START && code < CODE.INT_ZERO)
			) {
				return decodeInt(input, offset, code);
			}
			throw new Error(`unknown tuple code: 0x${code.toString(16)} at offset ${offset - 1}`);
	}
}

function decodeAll(input) {
	const out = [];
	let offset = 0;
	while (offset < input.length) {
		const r = decodeOne(input, offset, 0);
		out.push(r.value);
		offset = r.next;
	}
	return out;
}

function formatSegment(v) {
	if (v === null) return "nil";
	if (typeof v === "bigint") {
		const name = KEY_NAMES.get(Number(v));
		return name ? `${name} (${v.toString()})` : v.toString();
	}
	if (typeof v === "number") {
		const name = KEY_NAMES.get(v);
		return name ? `${name} (${v})` : v.toString();
	}
	if (typeof v === "string") return v;
	if (Array.isArray(v)) return `[${v.map(formatSegment).join(", ")}]`;
	if (v && typeof v === "object" && v.type === "bytes") return `bytes:${v.hex}`;
	if (v && typeof v === "object" && v.type === "versionstamp") return `vs(${v.dbVersion}:${v.batchVersion}:${v.userVersion})`;
	return String(v);
}

const segments = decodeAll(buf);
console.log(segments.map(formatSegment).join("/"));
