import { setup } from "rivetkit";
import { arenaMatch } from "./arena/match.ts";
import { arenaMatchmaker } from "./arena/matchmaker.ts";
import { battleRoyaleMatch } from "./battle-royale/match.ts";
import { battleRoyaleMatchmaker } from "./battle-royale/matchmaker.ts";
import { idleLeaderboard } from "./idle/leaderboard.ts";
import { idleWorld } from "./idle/world.ts";
import { ioStyleMatch } from "./io-style/match.ts";
import { ioStyleMatchmaker } from "./io-style/matchmaker.ts";
import { openWorldChunk } from "./open-world/chunk.ts";
import { partyMatch } from "./party/match.ts";
import { partyMatchmaker } from "./party/matchmaker.ts";
import { physics2dWorld } from "./physics-2d/world.ts";
import { physics3dWorld } from "./physics-3d/world.ts";
import { rankedLeaderboard } from "./ranked/leaderboard.ts";
import { rankedMatch } from "./ranked/match.ts";
import { rankedMatchmaker } from "./ranked/matchmaker.ts";
import { rankedPlayer } from "./ranked/player.ts";
import { turnBasedMatch } from "./turn-based/match.ts";
import { turnBasedMatchmaker } from "./turn-based/matchmaker.ts";

export {
	arenaMatch,
	arenaMatchmaker,
	battleRoyaleMatch,
	battleRoyaleMatchmaker,
	idleLeaderboard,
	idleWorld,
	ioStyleMatch,
	ioStyleMatchmaker,
	openWorldChunk,
	partyMatch,
	partyMatchmaker,
	physics2dWorld,
	physics3dWorld,
	rankedLeaderboard,
	rankedMatch,
	rankedMatchmaker,
	rankedPlayer,
	turnBasedMatch,
	turnBasedMatchmaker,
};

export const registry = setup({
	use: {
		arenaMatch,
		arenaMatchmaker,
		battleRoyaleMatch,
		battleRoyaleMatchmaker,
		idleLeaderboard,
		idleWorld,
		ioStyleMatchmaker,
		ioStyleMatch,
		openWorldChunk,
		partyMatch,
		partyMatchmaker,
		physics2dWorld,
		physics3dWorld,
		rankedLeaderboard,
		rankedMatch,
		rankedMatchmaker,
		rankedPlayer,
		turnBasedMatch,
		turnBasedMatchmaker,
	},
});
