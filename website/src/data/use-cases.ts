import {
	faClock,
	faDatabase,
	faDiagramNext,
	faFilePen,
	faGamepad,
	faGaugeHigh,
	faGlobe,
	faNetworkWired,
	faRobot,
	faRotate,
	faUserRobot,
} from "@rivet-gg/icons";

export interface UseCase {
	title: string;
	href: string;
	icon?: any;
	description?: string;
}

export const useCases: UseCase[] = [
	{
		title: "AI Agent",
		href: "/docs/use-cases/ai-agent",
		icon: faRobot,
		description:
			"Build durable AI assistants with persistent memory and realtime streaming",
	},
	{
		title: "Realtime Docs",
		href: "/docs/use-cases/crdt",
		icon: faFilePen,
		description:
			"Collaborative documents with CRDTs and realtime synchronization",
	},
	{
		title: "Workflows",
		href: "/docs/use-cases/workflows",
		icon: faDiagramNext,
		description:
			"Durable multi-step workflows with flexible state management",
	},
	{
		title: "Local-First Sync",
		href: "/docs/use-cases/sync",
		icon: faRotate,
		description: "Offline-first applications with server synchronization",
	},
	{
		title: "Bots",
		href: "/docs/use-cases/bots",
		icon: faUserRobot,
		description: "Discord, Slack, or autonomous bots with persistent state",
	},
	{
		title: "Per-Tenant Databases",
		href: "/docs/use-cases/database",
		icon: faDatabase,
		description:
			"Isolated data stores for each user with zero-latency access",
	},
	{
		title: "Multiplayer Game",
		href: "/docs/use-cases/game",
		icon: faGamepad,
		description:
			"Authoritative game servers with realtime state synchronization",
	},
	{
		title: "Background Jobs",
		href: "/docs/use-cases/background-jobs",
		icon: faClock,
		description:
			"Scheduled and recurring jobs without external queue infrastructure",
	},
	{
		title: "Rate Limiting",
		href: "/docs/use-cases/rate",
		icon: faGaugeHigh,
		description: "Distributed rate limiting with in-memory counters",
	},
	{
		title: "Geo-Distributed Database",
		href: "/docs/general/edge",
		icon: faGlobe,
		description:
			"Store data close to users globally with automatic edge distribution",
	},
	{
		title: "Agent Orchestration & MCP",
		href: "/docs/use-cases/ai-agent",
		icon: faRobot,
		description:
			"Build AI agents with persistence and stateful Model Context Protocol",
	},
	{
		title: "Multiplayer Apps",
		href: "/docs/use-cases/game",
		icon: faNetworkWired,
		description:
			"Build realtime multiplayer applications with server-authoritative state",
	},
];
