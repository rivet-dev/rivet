/**
 * Timing-safe comparison of two Uint8Arrays or strings.
 * This prevents timing attacks by always comparing all bytes.
 */
export function timingSafeEqual(
	a: Uint8Array | string,
	b: Uint8Array | string,
): boolean {
	const encoder = new TextEncoder();
	const bufferA = typeof a === "string" ? encoder.encode(a) : a;
	const bufferB = typeof b === "string" ? encoder.encode(b) : b;

	// Pad to max length to avoid leaking length information
	const maxLength = Math.max(bufferA.byteLength, bufferB.byteLength);
	let result = bufferA.byteLength ^ bufferB.byteLength;

	for (let i = 0; i < maxLength; i++) {
		const byteA = i < bufferA.byteLength ? bufferA[i] : 0;
		const byteB = i < bufferB.byteLength ? bufferB[i] : 0;
		result |= byteA ^ byteB;
	}

	return result === 0;
}
