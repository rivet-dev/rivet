// BARE decoder for epoxy CommittedValue (v2 and v3)
// Minimal implementation - no external dependencies

const hexString = process.argv[2];
if (!hexString) {
	console.error("Usage: node decode_epoxy_committed.js <hex_data>");
	console.error("Example: node decode_epoxy_committed.js 030000020000000000000001");
	process.exit(1);
}

const buffer = Buffer.from(hexString, "hex");

// First 2 bytes: VBARE version (u16 LE)
const version = buffer.readUInt16LE(0);
const dataBuffer = buffer.slice(2);

console.log("VBARE version:", version);
console.log("Decoding epoxy CommittedValue from hex:", hexString);

class BareDecoder {
	constructor(buf) {
		this.buffer = buf;
		this.offset = 0;
	}

	readByte() {
		return this.buffer[this.offset++];
	}

	// Variable-length unsigned integer (LEB128) - used for uint, list lengths, data lengths
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

	// Fixed 8-byte unsigned integer (u64, little-endian)
	readU64() {
		const lo = this.buffer.readUInt32LE(this.offset);
		const hi = this.buffer.readUInt32LE(this.offset + 4);
		this.offset += 8;
		// Return as BigInt for precision, then convert to number if safe
		const big = BigInt(hi) * BigInt(0x100000000) + BigInt(lo);
		return big <= BigInt(Number.MAX_SAFE_INTEGER) ? Number(big) : big;
	}

	// Length-prefixed byte array
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

const decoder = new BareDecoder(dataBuffer);
const result = {};

if (version === 2) {
	// v2: value is non-optional data
	result.value = decoder.readData();
	result.version = decoder.readU64();
	result.mutable = decoder.readBool();
} else if (version === 3) {
	// v3: value is optional<data>
	result.value = decoder.readOptional(() => decoder.readData());
	result.version = decoder.readU64();
	result.mutable = decoder.readBool();
} else {
	console.error("Unsupported VBARE version:", version);
	process.exit(1);
}

console.log("Decoded CommittedValue:");
console.log(
	JSON.stringify(
		result,
		(key, value) => {
			if (value && value.type === "Buffer") {
				return Buffer.from(value.data).toString("hex");
			}
			return value;
		},
		2,
	),
);

console.log("\nBytes consumed:", decoder.offset, "/", dataBuffer.length);
