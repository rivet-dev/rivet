import type { ActorConn } from "rivetkit/client";
import type {
	arenaMatch,
	arenaMatchmaker,
	battleRoyaleMatch,
	idleLeaderboard,
	idleWorld,
	ioStyleMatch,
	openWorldChunk,
	partyMatch,
	physics2dWorld,
	physics3dWorld,
	rankedLeaderboard,
	rankedMatch,
	rankedMatchmaker,
	turnBasedMatch,
	turnBasedMatchmaker,
} from "../src/actors/index.ts";

export type ArenaMatchConn = ActorConn<typeof arenaMatch>;
export type ArenaMatchmakerConn = ActorConn<typeof arenaMatchmaker>;
export type BattleRoyaleMatchConn = ActorConn<typeof battleRoyaleMatch>;
export type IdleLeaderboardConn = ActorConn<typeof idleLeaderboard>;
export type IdleWorldConn = ActorConn<typeof idleWorld>;
export type IoStyleMatchConn = ActorConn<typeof ioStyleMatch>;
export type OpenWorldChunkConn = ActorConn<typeof openWorldChunk>;
export type PartyMatchConn = ActorConn<typeof partyMatch>;
export type Physics2dConn = ActorConn<typeof physics2dWorld>;
export type Physics3dConn = ActorConn<typeof physics3dWorld>;
export type RankedLeaderboardConn = ActorConn<typeof rankedLeaderboard>;
export type RankedMatchConn = ActorConn<typeof rankedMatch>;
export type RankedMatchmakerConn = ActorConn<typeof rankedMatchmaker>;
export type TurnBasedMatchConn = ActorConn<typeof turnBasedMatch>;
export type TurnBasedMatchmakerConn = ActorConn<typeof turnBasedMatchmaker>;
