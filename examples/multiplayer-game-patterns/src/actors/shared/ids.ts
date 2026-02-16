function randomSuffix() {
	return Math.random().toString(36).slice(2, 8);
}

export function buildId(prefix: string): string {
	return `${prefix}-${Date.now()}-${randomSuffix()}`;
}

export function buildSecret(): string {
	return crypto.randomUUID().replaceAll("-", "");
}

export function buildPartyCode(length = 6): string {
	const alphabet = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
	let code = "";
	for (let i = 0; i < length; i++) {
		const idx = Math.floor(Math.random() * alphabet.length);
		code += alphabet[idx];
	}
	return code;
}
