// BARE decoder for namespace runner config v3
// Minimal implementation - no external dependencies

const hexString = process.argv[2];
if (!hexString) {
	console.error("Usage: node decode_runner_config.js <hex_data>");
	process.exit(1);
}

const buffer = Buffer.from(hexString, "hex");

// Skip version (first u16, 2 bytes)
const version = buffer.readUInt16LE(0);
const dataBuffer = buffer.slice(2);

console.log("Embedded VBARE Version:", version);

class BareDecoder {
	constructor(buffer) {
		this.buffer = buffer;
		this.offset = 0;
	}

	readByte() {
		return this.buffer[this.offset++];
	}

	readUint() {
		// Read variable-length unsigned integer (LEB128)
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

	readU32() {
		// Read fixed 32-bit unsigned integer (little-endian)
		const value = this.buffer.readUInt32LE(this.offset);
		this.offset += 4;
		return value;
	}

	readData() {
		// Read length-prefixed byte array
		const length = this.readUint();
		const data = this.buffer.slice(this.offset, this.offset + length);
		this.offset += length;
		return data;
	}

	readString() {
		// Read length-prefixed UTF-8 string
		return this.readData().toString("utf8");
	}

	readBool() {
		return this.readByte() !== 0;
	}

	readEnum() {
		return this.readUint();
	}

	readOptional(readFn) {
		const hasValue = this.readByte();
		if (hasValue === 0) return null;
		return readFn.call(this);
	}

	readMap(keyReadFn, valueReadFn) {
		const length = this.readUint();
		const map = {};
		for (let i = 0; i < length; i++) {
			const key = keyReadFn.call(this);
			const value = valueReadFn.call(this);
			map[key] = value;
		}
		return map;
	}

	readUnion(variants, tagNames) {
		const tag = this.readUint();
		const value = variants[tag].call(this);
		const result = {
			tag: tagNames ? tagNames[tag] : tag,
		};
		// Only include value if it's not void (undefined/null or the string representation)
		if (value !== undefined && value !== null && value !== "Normal") {
			result.value = value;
		}
		return result;
	}
}

// Decode the runner config
const decoder = new BareDecoder(dataBuffer);

console.log("Decoding runner config from hex:", hexString);

// RunnerConfig struct
const runnerConfig = {};

// kind: RunnerConfigKind union
const kind = decoder.readUnion(
	[
		// 0: Serverless
		() => {
			const serverless = {};
			serverless.url = decoder.readString();
			serverless.headers = decoder.readMap(
				() => decoder.readString(),
				() => decoder.readString(),
			);
			serverless.request_lifespan = decoder.readU32();
			serverless.slots_per_runner = decoder.readU32();
			serverless.min_runners = decoder.readU32();
			serverless.max_runners = decoder.readU32();
			serverless.runners_margin = decoder.readU32();
			return serverless;
		},
		// 1: Normal (void)
		() => "Normal",
	],
	["Serverless", "Normal"],
);
runnerConfig.kind = kind;

// metadata: optional<Json>
runnerConfig.metadata = decoder.readOptional(() => decoder.readString());

// drain_on_version_upgrade: bool
runnerConfig.drain_on_version_upgrade = decoder.readBool();

console.log("Decoded runner config:");
console.log(JSON.stringify(runnerConfig, null, 2));

console.log("\nBytes consumed:", decoder.offset, "/", dataBuffer.length);
