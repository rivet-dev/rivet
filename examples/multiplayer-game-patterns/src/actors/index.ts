import { setup } from "rivetkit";
import { asyncTurnBasedMatch } from "./turn-based/match.ts";
import { asyncTurnBasedMatchmaker } from "./turn-based/matchmaker.ts";
import { battleRoyaleMatch } from "./battle-royale/match.ts";
import { battleRoyaleMatchmaker } from "./battle-royale/matchmaker.ts";
import { competitiveMatch } from "./competitive/match.ts";
import { competitiveMatchmaker } from "./competitive/matchmaker.ts";
import { ioStyleMatch } from "./io-style/match.ts";
import { ioStyleMatchmaker } from "./io-style/matchmaker.ts";
import { openWorldChunk } from "./open-world/chunk.ts";
import { openWorldIndex } from "./open-world/world-index.ts";
import { partyMatch } from "./party/match.ts";
import { partyMatchmaker } from "./party/matchmaker.ts";
import { rankedMatch } from "./ranked/match.ts";
import { rankedMatchmaker } from "./ranked/matchmaker.ts";

export {
	asyncTurnBasedMatch,
	asyncTurnBasedMatchmaker,
	battleRoyaleMatch,
	battleRoyaleMatchmaker,
	competitiveMatch,
	competitiveMatchmaker,
	ioStyleMatch,
	ioStyleMatchmaker,
	openWorldChunk,
	openWorldIndex,
	partyMatch,
	partyMatchmaker,
	rankedMatch,
	rankedMatchmaker,
};

export const registry = setup({
	use: {
		ioStyleMatchmaker,
		ioStyleMatch,
		competitiveMatchmaker,
		competitiveMatch,
		partyMatchmaker,
		partyMatch,
		openWorldIndex,
		openWorldChunk,
		asyncTurnBasedMatchmaker,
		asyncTurnBasedMatch,
		rankedMatchmaker,
		rankedMatch,
		battleRoyaleMatchmaker,
		battleRoyaleMatch,
	},
});
