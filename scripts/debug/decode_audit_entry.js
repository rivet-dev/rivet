// BARE decoder for ACL audit entry
// Minimal implementation - no external dependencies

const hexString = process.argv[2];
if (!hexString) {
	console.error("Usage: node decode_audit_entry.js <hex_data>");
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

	readData() {
		// Read length-prefixed byte array
		const length = this.readUint();
		const data = this.buffer.slice(this.offset, this.offset + length);
		this.offset += length;
		return data;
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

	readUnion(variants, tagNames) {
		const tag = this.readUint();
		const value = variants[tag].call(this);
		const result = {
			tag: tagNames ? tagNames[tag] : tag,
		};
		// Only include value if it's not void (undefined/null or the string representation)
		if (value !== undefined && value !== null && value !== "Any") {
			result.value = value;
		}
		return result;
	}
}

// Decode the audit entry
const decoder = new BareDecoder(dataBuffer);

console.log("Decoding audit entry from hex:", hexString);

// Data struct
const data = {};

// AccessRequest
data.request = {};

// namespace: AccessNamespaceScope union
const namespace = decoder.readUnion(
	[
		() => "Any", // 0: Any (void)
		() => decoder.readData(), // 1: Id
		() => decoder.readData().toString("utf8"), // 2: Name
	],
	["Any", "Id", "Name"],
);
data.request.namespace = namespace;

// resource: ResourceKind enum
const resourceKinds = [
	"NAMESPACE",
	"ACTOR",
	"RUNNER",
	"RUNNER_CONFIG",
	"TOKEN",
	"ACL",
	"DATACENTER",
	"ACTOR_GATEWAY",
];
data.request.resource = resourceKinds[decoder.readEnum()];

// target: TargetScope union
const target = decoder.readUnion(
	[
		() => "Any", // 0: Any (void)
		() => decoder.readData(), // 1: Id
	],
	["Any", "Id"],
);
data.request.target = target;

// operation: OperationKind enum
const operationKinds = ["READ", "UPDATE", "LIST", "CREATE", "DELETE"];
data.request.operation = operationKinds[decoder.readEnum()];

// tokenId: optional<Id>
data.tokenId = decoder.readOptional(() => decoder.readData());

// allowed: bool
data.allowed = decoder.readBool();

console.log("Decoded audit entry:");
console.log(
	JSON.stringify(
		data,
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
