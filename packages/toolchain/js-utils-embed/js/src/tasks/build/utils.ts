export function encodeGlobalName(input: string): string {
	return input.replace(/[A-Z$]/g, (char) => `$${char.toLowerCase()}`);
}

export function decodeGlobalName(input: string): string {
	return input.replace(/\$[a-z$]/g, (encoded) => encoded[1].toUpperCase());
}