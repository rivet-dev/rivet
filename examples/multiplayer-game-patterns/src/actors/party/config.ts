export const MAX_PARTY_SIZE = 8;
export const PARTY_CODE_LENGTH = 6;

export type PartyPhase = "waiting" | "playing" | "finished";

// Characters that are unambiguous (no I/O/0/1).
const CODE_CHARS = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

export function generatePartyCode(): string {
	let code = "";
	for (let i = 0; i < PARTY_CODE_LENGTH; i++) {
		code += CODE_CHARS[Math.floor(Math.random() * CODE_CHARS.length)];
	}
	return code;
}

export function generatePlayerName(): string {
	return `Player#${Math.floor(Math.random() * 10000).toString().padStart(4, "0")}`;
}
