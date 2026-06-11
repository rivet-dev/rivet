export interface Tag {
	name: string;
	displayName: string;
}

export interface Technology {
	name: string;
	displayName: string;
}

export interface Template {
	name: string;
	displayName: string;
	description: string;
	tags: string[];
	technologies: string[];
	providers: {
		vercel: string | null;
	};
	noFrontend?: boolean;
}

// Local fallback registry for cookbook pages.
// Entries can be expanded as cookbook templates are added.
export const templates: Template[] = [
	{
		name: "ai-agent",
		displayName: "AI Agent",
		description:
			"Queue-driven AI agents where each Rivet Actor keeps its own conversation memory and streams AI SDK responses to the browser as events.",
		tags: ["ai", "realtime"],
		technologies: ["react", "ai-sdk"],
		providers: { vercel: null },
	},
	{
		name: "chat-room",
		displayName: "Chat Room",
		description:
			"Realtime chat where each room is its own Rivet Actor, persisting message history in the actor's SQLite database and broadcasting new messages to connected clients.",
		tags: ["realtime"],
		technologies: ["react", "sqlite", "websocket"],
		providers: { vercel: null },
	},
	{
		name: "collaborative-document",
		displayName: "Collaborative Document",
		description:
			"A shared text editor where each document is a Rivet Actor that relays Yjs CRDT updates and awareness to every collaborator and persists the merged snapshot.",
		tags: ["realtime", "collaboration"],
		technologies: ["react", "yjs", "websocket"],
		providers: { vercel: null },
	},
	{
		name: "cursors",
		displayName: "Live Cursors",
		description:
			"Tracks live cursor positions per connection with connection state and broadcasts moves to every client as typed events.",
		tags: ["realtime", "collaboration"],
		technologies: ["react", "websocket"],
		providers: { vercel: null },
	},
	{
		name: "cursors-raw-websocket",
		displayName: "Live Cursors (Raw WebSocket)",
		description:
			"A collaborative cursor room built on the raw WebSocket handler with a custom JSON message protocol.",
		tags: ["realtime", "collaboration"],
		technologies: ["react", "websocket"],
		providers: { vercel: null },
	},
	{
		name: "per-tenant-database",
		displayName: "Per-Tenant Database",
		description:
			"Each tenant maps to its own Rivet Actor keyed by tenant id, so actor state acts as a fully isolated per-tenant database with live updates.",
		tags: ["data", "realtime"],
		technologies: ["react"],
		providers: { vercel: null },
	},
	{
		name: "sqlite-drizzle",
		displayName: "SQLite + Drizzle",
		description:
			"Drizzle ORM with actor-scoped SQLite using a typed schema and generated migrations.",
		tags: ["data"],
		technologies: ["sqlite"],
		providers: { vercel: null },
		noFrontend: true,
	},
	{
		name: "scheduling",
		displayName: "Scheduling",
		description:
			"Schedule actor actions to run after a delay or at an exact timestamp, with durable timers that survive actor restarts.",
		tags: ["jobs"],
		technologies: ["react"],
		providers: { vercel: null },
	},
	{
		name: "sandbox-coding-agent",
		displayName: "Sandbox Coding Agent",
		description:
			"Each agent actor drives a coding agent running inside an isolated sandbox, with Docker, Daytona, or E2B providers.",
		tags: ["ai"],
		technologies: ["react"],
		providers: { vercel: null },
	},
];

export const TAGS: Tag[] = [
	{ name: "ai", displayName: "AI" },
	{ name: "realtime", displayName: "Realtime" },
	{ name: "collaboration", displayName: "Collaboration" },
	{ name: "data", displayName: "Data" },
	{ name: "jobs", displayName: "Jobs" },
];

export const TECHNOLOGIES: Technology[] = [
	{ name: "react", displayName: "React" },
	{ name: "sqlite", displayName: "SQLite" },
	{ name: "yjs", displayName: "Yjs" },
	{ name: "ai-sdk", displayName: "AI SDK" },
	{ name: "websocket", displayName: "WebSocket" },
];
