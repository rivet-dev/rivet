// BARE decoder for epoxy ChangelogEntry (raw, no vbare header).
// Detects v2 (value: data) vs v3 (value: optional<data>) via roundtrip check.
// See engine/sdks/schemas/epoxy-protocol/{v2,v3}.bare
// Minimal implementation - no external dependencies

const hexString = process.argv[2];
if (!hexString) {
	console.error("Usage: node decode_epoxy_changelog.js <hex_data>");
	process.exit(1);
}

const buffer = Buffer.from(hexString, "hex");

class BareDecoder {
	constructor(buffer) {
		this.buffer = buffer;
		this.offset = 0;
	}

	readByte() {
		return this.buffer[this.offset++];
	}

	readUint() {
		let result = 0;
		let shift = 0;
		while (true) {
			const byte = this.readByte();
			result |= (byte & 0x7f) << shift;
			if ((byte & 0x80) === 0) break;
			shift += 7;
		}
		return result;
	}

	readU64() {
		const value = this.buffer.readBigUInt64LE(this.offset);
		this.offset += 8;
		return value;
	}

	readData() {
		const length = this.readUint();
		const data = this.buffer.slice(this.offset, this.offset + length);
		this.offset += length;
		return data;
	}

	readBool() {
		return this.readByte() !== 0;
	}

	readOptional(readFn) {
		const hasValue = this.readByte();
		if (hasValue === 0) return null;
		return readFn.call(this);
	}
}

function encodeUleb128(n) {
	if (n === 0) return Buffer.from([0x00]);
	const out = [];
	while (n) {
		let b = n & 0x7f;
		n >>>= 7;
		if (n) b |= 0x80;
		out.push(b);
	}
	return Buffer.from(out);
}

function encodeData(b) {
	return Buffer.concat([encodeUleb128(b.length), b]);
}

// Try v2: key: data, value: data, version: u64, mutable: bool.
// Confirmed via roundtrip re-encode.
function tryDecodeV2(buf) {
	try {
		const d = new BareDecoder(buf);
		const key = d.readData();
		const value = d.readData();
		if (d.offset + 9 !== buf.length) return null;
		const version = d.readU64();
		const mutable = d.readBool();
		const roundtrip = Buffer.concat([encodeData(key), encodeData(value), buf.slice(d.offset - 9)]);
		if (!roundtrip.equals(buf)) return null;
		return { schema: "v2", key, value, version, mutable };
	} catch {
		return null;
	}
}

// Try v3: key: data, value: optional<data>, version: u64, mutable: bool.
function tryDecodeV3(buf) {
	try {
		const d = new BareDecoder(buf);
		const key = d.readData();
		const value = d.readOptional(() => d.readData());
		if (d.offset + 9 !== buf.length) return null;
		const version = d.readU64();
		const mutable = d.readBool();
		return { schema: "v3", key, value, version, mutable };
	} catch {
		return null;
	}
}

const v2 = tryDecodeV2(buffer);
const v3 = tryDecodeV3(buffer);
const entry = v2 ?? v3;

if (!entry) {
	console.error("Could not decode as v2 or v3 ChangelogEntry");
	process.exit(1);
}

const ambiguous = v2 !== null && v3 !== null;

console.log("Decoding epoxy ChangelogEntry from hex:", hexString);
console.log("");
console.log("Decoded ChangelogEntry:");
console.log(
	JSON.stringify(
		{
			schema: entry.schema + (ambiguous ? " (ambiguous: also valid as v3)" : ""),
			key: entry.key.toString("hex"),
			value: entry.value === null ? null : entry.value.toString("hex"),
			version: entry.version.toString(),
			mutable: entry.mutable,
		},
		null,
		2,
	),
);
