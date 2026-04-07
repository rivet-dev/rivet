export class KvStorageQuotaExceededError extends Error {
	readonly remaining: number;
	readonly payloadSize: number;

	constructor(remaining: number, payloadSize: number) {
		super(
			`not enough space left in storage (${remaining} bytes remaining, current payload is ${payloadSize} bytes)`,
		);
		this.name = "KvStorageQuotaExceededError";
		this.remaining = remaining;
		this.payloadSize = payloadSize;
	}
}
