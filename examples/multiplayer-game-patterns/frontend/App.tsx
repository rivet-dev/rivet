import { useEffect, useMemo, useState } from "react";
import type { LucideIcon } from "lucide-react";
import {
	Globe,
	Swords,
	Users,
	Grid3X3,
	Trophy,
	Skull,
	Map,
	Factory,
	Box,
	Boxes,
} from "lucide-react";
import { makeClient } from "./client.ts";
import { ArenaMenu } from "./games/arena/menu.tsx";
import { ArenaGameView } from "./games/arena/game.tsx";
import { BattleRoyaleMenu } from "./games/battle-royale/menu.tsx";
import { BattleRoyaleGameView } from "./games/battle-royale/game.tsx";
import { IdleMenu } from "./games/idle/menu.tsx";
import { IdleGame } from "./games/idle/game.tsx";
import { IoStyleMenu } from "./games/io-style/menu.tsx";
import { IoStyleGame } from "./games/io-style/game.tsx";
import { OpenWorldMenu } from "./games/open-world/menu.tsx";
import { OpenWorldGameView } from "./games/open-world/game.tsx";
import { PartyMenu } from "./games/party/menu.tsx";
import { PartyGame } from "./games/party/game.tsx";
import { Physics2dMenu } from "./games/physics-2d/menu.tsx";
import { Physics2dGameView } from "./games/physics-2d/game.tsx";
import { Physics3dMenu } from "./games/physics-3d/menu.tsx";
import { Physics3dGameView } from "./games/physics-3d/game.tsx";
import { RankedMenu } from "./games/ranked/menu.tsx";
import { RankedGameView } from "./games/ranked/game.tsx";
import { TurnBasedMenu } from "./games/turn-based/menu.tsx";
import { TurnBasedGame } from "./games/turn-based/game.tsx";

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
	| { page: "game"; pattern: PatternId; matchInfo: unknown };

const PATTERNS: Array<{ id: PatternId; title: string; description: string; icon: LucideIcon }> = [
	{
		id: "physics-2d",
		title: "Physics 2D",
		description: "Shared Rapier 2D physics at 10 TPS with client-side prediction and network smoothing.",
		icon: Box,
	},
	{
		id: "physics-3d",
		title: "Physics 3D",
		description: "Shared Rapier 3D physics at 10 TPS with Three.js rendering and network smoothing.",
		icon: Boxes,
	},
	{
		id: "io-style",
		title: "IO-Style",
		description: "Open lobby matchmaking with server-authoritative movement at 10 tps.",
		icon: Globe,
	},
	{
		id: "arena",
		title: "Arena",
		description: "Mode-based fixed-capacity matches with hybrid netcode and hitscan combat at 20 tps.",
		icon: Swords,
	},
	{
		id: "battle-royale",
		title: "Battle Royale",
		description: "Waiting lobby into shrinking zone gameplay. Last player standing wins.",
		icon: Skull,
	},
	{
		id: "ranked",
		title: "Ranked",
		description: "1v1 ELO-based matchmaking with expanding rating windows. First to 5 kills.",
		icon: Trophy,
	},
	{
		id: "open-world",
		title: "Open World",
		description: "Infinite chunk-based world with cross-chunk movement and coordinate routing.",
		icon: Map,
	},
	{
		id: "idle",
		title: "Idle",
		description: "Offline progression with scheduled production, building management, and global leaderboard.",
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
		description: "Event-driven party lobby with invite codes and host controls.",
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
					<div key={p.id} className="card" onClick={() => onSelect(p.id)}>
						<h3><p.icon size={18} style={{ verticalAlign: "middle", marginRight: 8, opacity: 0.7 }} />{p.title}</h3>
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
	const goGame = (pattern: PatternId, matchInfo: unknown) =>
		setRoute({ page: "game", pattern, matchInfo });

	if (route.page === "selector") {
		return <Selector onSelect={goMenu} />;
	}

	if (route.page === "menu") {
		const props = {
			client,
			onReady: (info: unknown) => goGame(route.pattern, info),
			onBack: goSelector,
		};
		switch (route.pattern) {
			case "io-style":
				return <IoStyleMenu {...props} />;
			case "arena":
				return <ArenaMenu {...props} />;
			case "party":
				return <PartyMenu {...props} />;
			case "turn-based":
				return <TurnBasedMenu {...props} />;
			case "ranked":
				return <RankedMenu {...props} />;
			case "battle-royale":
				return <BattleRoyaleMenu {...props} />;
			case "open-world":
				return <OpenWorldMenu {...props} />;
			case "idle":
				return <IdleMenu {...props} />;
			case "physics-2d":
				return <Physics2dMenu {...props} />;
			case "physics-3d":
				return <Physics3dMenu {...props} />;
		}
	}

	if (route.page === "game") {
		const props = {
			client,
			matchInfo: route.matchInfo as never,
			onLeave: goSelector,
		};
		switch (route.pattern) {
			case "io-style":
				return <IoStyleGame {...props} />;
			case "arena":
				return <ArenaGameView {...props} />;
			case "party":
				return <PartyGame {...props} />;
			case "turn-based":
				return <TurnBasedGame {...props} />;
			case "ranked":
				return <RankedGameView {...props} />;
			case "battle-royale":
				return <BattleRoyaleGameView {...props} />;
			case "open-world":
				return <OpenWorldGameView {...props} />;
			case "idle":
				return <IdleGame {...props} />;
			case "physics-2d":
				return <Physics2dGameView {...props} />;
			case "physics-3d":
				return <Physics3dGameView {...props} />;
		}
	}

	return null;
}
