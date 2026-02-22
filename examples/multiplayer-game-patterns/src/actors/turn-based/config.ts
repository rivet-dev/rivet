export const BOARD_SIZE = 3;
export const INVITE_CODE_LENGTH = 6;

export type CellValue = "" | "X" | "O";
export type GameResult = "x_wins" | "o_wins" | "draw" | null;

const CODE_CHARS = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

export function generateInviteCode(): string {
	let code = "";
	for (let i = 0; i < INVITE_CODE_LENGTH; i++) {
		code += CODE_CHARS[Math.floor(Math.random() * CODE_CHARS.length)];
	}
	return code;
}
