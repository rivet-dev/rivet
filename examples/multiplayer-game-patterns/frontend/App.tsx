import type { LucideIcon } from "lucide-react";
import {
	Box,
	Boxes,
	Factory,
	Globe,
	Grid3X3,
	Map,
	Skull,
	Swords,
	Trophy,
	Users,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { makeClient } from "./client.ts";
import { ArenaGameView } from "./games/arena/game.tsx";
import { ArenaMenu, type ArenaMatchInfo } from "./games/arena/menu.tsx";
import { BattleRoyaleGameView } from "./games/battle-royale/game.tsx";
import {
	BattleRoyaleMenu,
	type BattleRoyaleMatchInfo,
} from "./games/battle-royale/menu.tsx";
import { IdleGame } from "./games/idle/game.tsx";
import { IdleMenu, type IdleMatchInfo } from "./games/idle/menu.tsx";
import { IoStyleGame } from "./games/io-style/game.tsx";
import { IoStyleMenu, type IoStyleMatchInfo } from "./games/io-style/menu.tsx";
import { OpenWorldGameView } from "./games/open-world/game.tsx";
import {
	OpenWorldMenu,
	type OpenWorldMatchInfo,
} from "./games/open-world/menu.tsx";
import { PartyGame } from "./games/party/game.tsx";
import { PartyMenu, type PartyMatchInfo } from "./games/party/menu.tsx";
import { Physics2dGameView } from "./games/physics-2d/game.tsx";
import {
	Physics2dMenu,
	type Physics2dMatchInfo,
} from "./games/physics-2d/menu.tsx";
import { Physics3dGameView } from "./games/physics-3d/game.tsx";
import {
	Physics3dMenu,
	type Physics3dMatchInfo,
} from "./games/physics-3d/menu.tsx";
import { RankedGameView } from "./games/ranked/game.tsx";
import { RankedMenu, type RankedMatchInfo } from "./games/ranked/menu.tsx";
import { TurnBasedGame } from "./games/turn-based/game.tsx";
import {
	TurnBasedMenu,
	type TurnBasedMatchInfo,
} from "./games/turn-based/menu.tsx";

type PatternId =
	| "io-style"
	| "arena"
	| "party"
	| "turn-based"
	| "ranked"
	| "battle-royale"
	| "open-world"
	| "idle"
	| "physics-2d"
	| "physics-3d";

type Route =
	| { page: "selector" }
	| { page: "menu"; pattern: PatternId }
	| { page: "game"; pattern: "io-style"; matchInfo: IoStyleMatchInfo }
	| { page: "game"; pattern: "arena"; matchInfo: ArenaMatchInfo }
	| { page: "game"; pattern: "party"; matchInfo: PartyMatchInfo }
	| { page: "game"; pattern: "turn-based"; matchInfo: TurnBasedMatchInfo }
	| { page: "game"; pattern: "ranked"; matchInfo: RankedMatchInfo }
	| {
			page: "game";
			pattern: "battle-royale";
			matchInfo: BattleRoyaleMatchInfo;
	  }
	| { page: "game"; pattern: "open-world"; matchInfo: OpenWorldMatchInfo }
	| { page: "game"; pattern: "idle"; matchInfo: IdleMatchInfo }
	| { page: "game"; pattern: "physics-2d"; matchInfo: Physics2dMatchInfo }
	| { page: "game"; pattern: "physics-3d"; matchInfo: Physics3dMatchInfo };

const PATTERNS: Array<{
	id: PatternId;
	title: string;
	description: string;
	icon: LucideIcon;
}> = [
	{
		id: "physics-2d",
		title: "Physics 2D",
		description:
			"Shared Rapier 2D physics at 10 TPS with client-side prediction and network smoothing.",
		icon: Box,
	},
	{
		id: "physics-3d",
		title: "Physics 3D",
		description:
			"Shared Rapier 3D physics at 10 TPS with Three.js rendering and network smoothing.",
		icon: Boxes,
	},
	{
		id: "io-style",
		title: "IO-Style",
		description:
			"Open lobby matchmaking with server-authoritative movement at 10 tps.",
		icon: Globe,
	},
	{
		id: "arena",
		title: "Arena",
		description:
			"Mode-based fixed-capacity matches with hybrid netcode and hitscan combat at 20 tps.",
		icon: Swords,
	},
	{
		id: "battle-royale",
		title: "Battle Royale",
		description:
			"Waiting lobby into shrinking zone gameplay. Last player standing wins.",
		icon: Skull,
	},
	{
		id: "ranked",
		title: "Ranked",
		description:
			"1v1 ELO-based matchmaking with expanding rating windows. First to 5 kills.",
		icon: Trophy,
	},
	{
		id: "open-world",
		title: "Open World",
		description:
			"Infinite chunk-based world with cross-chunk movement and coordinate routing.",
		icon: Map,
	},
	{
		id: "idle",
		title: "Idle",
		description:
			"Offline progression with scheduled production, building management, and global leaderboard.",
		icon: Factory,
	},
	{
		id: "turn-based",
		title: "Turn-Based",
		description: "Tic-tac-toe with invite codes and open matchmaking pool.",
		icon: Grid3X3,
	},
	{
		id: "party",
		title: "Party",
		description:
			"Event-driven party lobby with invite codes and host controls.",
		icon: Users,
	},
];

function Selector({ onSelect }: { onSelect: (id: PatternId) => void }) {
	return (
		<div className="app">
			<div className="page-header">
				<h1>Multiplayer Game Patterns</h1>
				<p>Select a matchmaking pattern to try.</p>
			</div>
			<div className="card-grid">
				{PATTERNS.map((p) => (
					<div
						key={p.id}
						className="card"
						onClick={() => onSelect(p.id)}
					>
						<h3>
							<p.icon
								size={18}
								style={{
									verticalAlign: "middle",
									marginRight: 8,
									opacity: 0.7,
								}}
							/>
							{p.title}
						</h3>
						<p>{p.description}</p>
					</div>
				))}
			</div>
		</div>
	);
}

export function App() {
	const client = useMemo(() => makeClient(), []);
	const [route, setRoute] = useState<Route>({ page: "selector" });

	useEffect(() => () => void client.dispose(), [client]);

	const goSelector = () => setRoute({ page: "selector" });
	const goMenu = (pattern: PatternId) => setRoute({ page: "menu", pattern });

	if (route.page === "selector") {
		return <Selector onSelect={goMenu} />;
	}

	if (route.page === "menu") {
		switch (route.pattern) {
			case "io-style":
				return (
					<IoStyleMenu
						client={client}
						onReady={(matchInfo) =>
							setRoute({
								page: "game",
								pattern: "io-style",
								matchInfo,
							})
						}
						onBack={goSelector}
					/>
				);
			case "arena":
				return (
					<ArenaMenu
						client={client}
						onReady={(matchInfo) =>
							setRoute({
								page: "game",
								pattern: "arena",
								matchInfo,
							})
						}
						onBack={goSelector}
					/>
				);
			case "party":
				return (
					<PartyMenu
						client={client}
						onReady={(matchInfo) =>
							setRoute({
								page: "game",
								pattern: "party",
								matchInfo,
							})
						}
						onBack={goSelector}
					/>
				);
			case "turn-based":
				return (
					<TurnBasedMenu
						client={client}
						onReady={(matchInfo) =>
							setRoute({
								page: "game",
								pattern: "turn-based",
								matchInfo,
							})
						}
						onBack={goSelector}
					/>
				);
			case "ranked":
				return (
					<RankedMenu
						client={client}
						onReady={(matchInfo) =>
							setRoute({
								page: "game",
								pattern: "ranked",
								matchInfo,
							})
						}
						onBack={goSelector}
					/>
				);
			case "battle-royale":
				return (
					<BattleRoyaleMenu
						client={client}
						onReady={(matchInfo) =>
							setRoute({
								page: "game",
								pattern: "battle-royale",
								matchInfo,
							})
						}
						onBack={goSelector}
					/>
				);
			case "open-world":
				return (
					<OpenWorldMenu
						client={client}
						onReady={(matchInfo) =>
							setRoute({
								page: "game",
								pattern: "open-world",
								matchInfo,
							})
						}
						onBack={goSelector}
					/>
				);
			case "idle":
				return (
					<IdleMenu
						client={client}
						onReady={(matchInfo) =>
							setRoute({
								page: "game",
								pattern: "idle",
								matchInfo,
							})
						}
						onBack={goSelector}
					/>
				);
			case "physics-2d":
				return (
					<Physics2dMenu
						client={client}
						onReady={(matchInfo) =>
							setRoute({
								page: "game",
								pattern: "physics-2d",
								matchInfo,
							})
						}
						onBack={goSelector}
					/>
				);
			case "physics-3d":
				return (
					<Physics3dMenu
						client={client}
						onReady={(matchInfo) =>
							setRoute({
								page: "game",
								pattern: "physics-3d",
								matchInfo,
							})
						}
						onBack={goSelector}
					/>
				);
		}
	}

	if (route.page === "game") {
		switch (route.pattern) {
			case "io-style":
				return (
					<IoStyleGame
						client={client}
						matchInfo={route.matchInfo}
						onLeave={goSelector}
					/>
				);
			case "arena":
				return (
					<ArenaGameView
						client={client}
						matchInfo={route.matchInfo}
						onLeave={goSelector}
					/>
				);
			case "party":
				return (
					<PartyGame
						client={client}
						matchInfo={route.matchInfo}
						onLeave={goSelector}
					/>
				);
			case "turn-based":
				return (
					<TurnBasedGame
						client={client}
						matchInfo={route.matchInfo}
						onLeave={goSelector}
					/>
				);
			case "ranked":
				return (
					<RankedGameView
						client={client}
						matchInfo={route.matchInfo}
						onLeave={goSelector}
					/>
				);
			case "battle-royale":
				return (
					<BattleRoyaleGameView
						client={client}
						matchInfo={route.matchInfo}
						onLeave={goSelector}
					/>
				);
			case "open-world":
				return (
					<OpenWorldGameView
						client={client}
						matchInfo={route.matchInfo}
						onLeave={goSelector}
					/>
				);
			case "idle":
				return (
					<IdleGame
						client={client}
						matchInfo={route.matchInfo}
						onLeave={goSelector}
					/>
				);
			case "physics-2d":
				return (
					<Physics2dGameView
						client={client}
						matchInfo={route.matchInfo}
						onLeave={goSelector}
					/>
				);
			case "physics-3d":
				return (
					<Physics3dGameView
						client={client}
						matchInfo={route.matchInfo}
						onLeave={goSelector}
					/>
				);
		}
	}

	return null;
}
