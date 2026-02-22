const PLAYER_COLOR_PALETTE = [
	"#ff6b6b",
	"#4ecdc4",
	"#45b7d1",
	"#f7b801",
	"#5c7cfa",
	"#20c997",
	"#f06595",
	"#ffa94d",
	"#74c0fc",
	"#94d82d",
	"#e599f7",
	"#ffd43b",
];

export function getPlayerColor(playerId: string): string {
	let hash = 0;
	for (let i = 0; i < playerId.length; i++) {
		hash = (hash * 31 + playerId.charCodeAt(i)) | 0;
	}
	const index = ((hash % PLAYER_COLOR_PALETTE.length) + PLAYER_COLOR_PALETTE.length) % PLAYER_COLOR_PALETTE.length;
	return PLAYER_COLOR_PALETTE[index]!;
}
