// Fixture catalog for the cluster onboarding + dashboard mock. Everything in
// here is invented for design exploration; nothing is read from the API.

export type SizeId =
	| "r-10"
	| "r-20"
	| "r-40"
	| "r-80"
	| "r-160"
	| "r-320"
	| "r-640"
	| "r-1280";

export interface NodeSize {
	id: SizeId;
	/** Hardware description shown to the user, such as `2 vCPU / 8 GB`. */
	label: string;
	/** vCPU per node. The two smallest sizes are shared-core, so this is fractional. */
	vcpu: number;
	memoryMb: number;
	/**
	 * Local NVMe in front of S3 tiered storage, GB. Writes land here as a
	 * write-ahead log before they are flushed to S3, and reads back from S3
	 * are cached here. Only the control plane uses it, so only the plan
	 * table shows it.
	 */
	diskCacheGb: number;
	/** Sustained write throughput of that disk, MB/s. Cluster throughput is this times the node count. */
	writeMbps: number;
	/** Monthly price per node on AWS, USD. */
	monthlyUsd: number;
}

function nodeSize(
	id: SizeId,
	vcpu: number,
	memoryGb: number,
	writeMbps: number,
	monthlyUsd: number,
): NodeSize {
	return {
		id,
		label: `${formatVcpu(vcpu)} vCPU / ${memoryGb} GB`,
		vcpu,
		memoryMb: memoryGb * 1024,
		diskCacheGb: memoryGb * 10,
		writeMbps,
		monthlyUsd,
	};
}

export function formatVcpu(vcpu: number): string {
	if (vcpu === 0.25) return "¼";
	if (vcpu === 0.5) return "½";
	return `${vcpu}`;
}

// Size ids count memory in 100 MB units; the user only ever sees the hardware
// label. The ladder mirrors GCP's E2 line: two shared-core sizes at the
// bottom, then whole cores at 4 GB per vCPU, with prices roughly doubling per
// step.
export const NODE_SIZES: NodeSize[] = [
	nodeSize("r-10", 0.25, 1, 30, 7),
	nodeSize("r-20", 0.5, 2, 60, 13),
	nodeSize("r-40", 1, 4, 125, 25),
	nodeSize("r-80", 2, 8, 250, 50),
	nodeSize("r-160", 4, 16, 500, 100),
	nodeSize("r-320", 8, 32, 1000, 195),
	nodeSize("r-640", 16, 64, 2000, 380),
	nodeSize("r-1280", 32, 128, 4000, 740),
];

export type TierId = "pro" | "team" | "enterprise";

/**
 * One control plane configuration a plan can be bought in. Named like
 * Aiven's plans after the RAM per node: `Startup-8` is the Startup plan on
 * 8 GB nodes. Business also varies the node count, so `Business-7x-64` is 7
 * nodes with 64 GB each.
 */
export interface PlanOption {
	id: string;
	label: string;
	nodes: number;
	size: SizeId;
	/** Monthly price on AWS, USD. */
	monthlyUsd: number;
}

export interface Tier {
	id: TierId;
	label: string;
	description: string;
	/** Control plane configurations, smallest first. */
	options: PlanOption[];
	/** Control plane option selected when switching to this plan, a `PlanOption` id. */
	defaultPlan: string;
	/** Largest runner node size on this plan. */
	maxSize: SizeId;
	/** Fewest runner nodes on this plan. 3 on multi-zone plans so runners cover every zone. */
	minRunners: number;
	/** Most runner nodes on this plan, or undefined when the plan has no limit. */
	maxRunners?: number;
	/** Runner config selected when switching to this plan. */
	defaultRunners: { size: SizeId; count: number };
	/** Included actor storage, GB. */
	storageGb: number;
	highlights: string[];
}

function planOption(
	tier: TierId,
	label: string,
	nodes: number,
	size: SizeId,
	monthlyUsd: number,
	naming: "ram" | "nodes-ram" = "ram",
): PlanOption {
	const memoryMb = NODE_SIZES.find((s) => s.id === size)?.memoryMb ?? 0;
	const gb = memoryMb / 1024;
	const shape = naming === "ram" ? `${gb}` : `${nodes}x-${gb}`;
	return {
		id: `${tier}-${shape}`,
		label: `${label}-${shape}`,
		nodes,
		size,
		monthlyUsd,
	};
}

// Plans are grouped by availability zone coverage: one node in one zone, a 3
// node set across 3 zones, or 5+ nodes across 3 zones. Within a plan you pick
// how big each node is. Rivet scales horizontally by adding Actor Nodes and
// keeps actor data in S3 tiered storage, so the control plane's disk is a
// cache rather than the storage limit.
export const TIERS: Tier[] = [
	{
		id: "pro",
		label: "Hobby",
		description: "For prototypes, side projects, and staging.",
		options: [
			planOption("pro", "Hobby", 1, "r-10", 10),
			planOption("pro", "Hobby", 1, "r-20", 18),
			planOption("pro", "Hobby", 1, "r-40", 32),
			planOption("pro", "Hobby", 1, "r-80", 60),
		],
		defaultPlan: "pro-1",
		maxSize: "r-80",
		minRunners: 1,
		maxRunners: 3,
		defaultRunners: { size: "r-10", count: 1 },
		storageGb: 20,
		highlights: [
			"1 control plane node in 1 availability zone",
			"Up to 3 Actor Nodes",
			"Email support",
		],
	},
	{
		id: "team",
		label: "Startup",
		description: "For production workloads.",
		options: [
			planOption("team", "Startup", 3, "r-40", 120),
			planOption("team", "Startup", 3, "r-80", 220),
			planOption("team", "Startup", 3, "r-160", 420),
			planOption("team", "Startup", 3, "r-320", 800),
			planOption("team", "Startup", 3, "r-640", 1550),
			planOption("team", "Startup", 3, "r-1280", 3000),
		],
		defaultPlan: "team-4",
		maxSize: "r-1280",
		minRunners: 3,
		maxRunners: 20,
		defaultRunners: { size: "r-80", count: 3 },
		storageGb: 100,
		highlights: [
			"3 control plane nodes across 3 availability zones",
			"Up to 20 Actor Nodes",
			"Slack & email support",
		],
	},
	{
		id: "enterprise",
		label: "Business",
		description: "For business-critical production at scale.",
		options: [
			planOption("enterprise", "Business", 5, "r-160", 900, "nodes-ram"),
			planOption("enterprise", "Business", 5, "r-320", 1700, "nodes-ram"),
			planOption("enterprise", "Business", 5, "r-640", 3300, "nodes-ram"),
			planOption("enterprise", "Business", 7, "r-640", 4500, "nodes-ram"),
			planOption("enterprise", "Business", 9, "r-640", 5700, "nodes-ram"),
			planOption(
				"enterprise",
				"Business",
				9,
				"r-1280",
				11000,
				"nodes-ram",
			),
		],
		defaultPlan: "enterprise-5x-16",
		maxSize: "r-1280",
		minRunners: 3,
		defaultRunners: { size: "r-80", count: 3 },
		storageGb: 500,
		highlights: [
			"5 to 9 control plane nodes across 3 availability zones",
			"Unlimited Actor Nodes",
			"Slack & email support",
		],
	},
];

export type CloudId = "aws" | "gcp";

export interface Region {
	id: string;
	city: string;
	/** ISO 3166-1 alpha-2, used to render a flag emoji. */
	country: string;
}

export interface Cloud {
	id: CloudId;
	label: string;
	description: string;
	/** Price multiplier against the AWS baseline. */
	multiplier: number;
	regions: Region[];
}

export const CLOUDS: Cloud[] = [
	{
		id: "aws",
		label: "AWS",
		description: "Graviton instances in 7 regions.",
		multiplier: 1,
		regions: [
			{ id: "us-east-1", city: "N. Virginia", country: "US" },
			{ id: "us-west-2", city: "Oregon", country: "US" },
			{ id: "eu-west-1", city: "Ireland", country: "IE" },
			{
				id: "eu-central-1",
				city: "Frankfurt",
				country: "DE",
			},
			{
				id: "ap-southeast-1",
				city: "Singapore",
				country: "SG",
			},
			{ id: "ap-northeast-1", city: "Tokyo", country: "JP" },
			{ id: "sa-east-1", city: "São Paulo", country: "BR" },
		],
	},
	{
		id: "gcp",
		label: "Google Cloud",
		description: "Tau T2A machine family in 6 regions.",
		multiplier: 1.05,
		regions: [
			{ id: "us-central1", city: "Iowa", country: "US" },
			{ id: "us-east4", city: "N. Virginia", country: "US" },
			{ id: "europe-west1", city: "Belgium", country: "BE" },
			{
				id: "europe-west3",
				city: "Frankfurt",
				country: "DE",
			},
			{ id: "asia-east1", city: "Taiwan", country: "TW" },
			{
				id: "asia-southeast1",
				city: "Singapore",
				country: "SG",
			},
		],
	},
];

/** Billing constants shared by the checkout rail and the dashboard. */
export const BILLING = {
	includedEgressGb: 100,
	egressUsdPerGb: 0.06,
	extraStorageUsdPerGb: 0.25,
	/** S3-compatible object storage included per GB of actor storage. */
	objectStorageMultiplier: 5,
	extraObjectStorageUsdPerGb: 0.02,
} as const;

export type UseCaseId =
	| "chat"
	| "ai-agent"
	| "multiplayer-game"
	| "collab-docs"
	| "custom";

export interface RecommendedActor {
	name: string;
	/** Key pattern, e.g. `room:{roomId}`. */
	key: string;
	description: string;
}

export interface Improvement {
	title: string;
	description: string;
	/** Prompt handed to the coding agent. */
	prompt: string;
	/** Which part of the dashboard it touches, used for the icon. */
	area: "security" | "reliability" | "scale" | "observability" | "product";
	/** Rivet product the improvement is built on, shown as a badge. */
	product?: RivetProduct;
}

/** Rivet products beyond plain Actors, as named on rivet.dev. */
export type RivetProduct =
	| "Workflows"
	| "agentOS"
	| "Secure Exec"
	| "Dynamic Apps";

export type DashboardWidget =
	| { kind: "stat"; title: string; value: string; delta?: string }
	| { kind: "chart"; title: string; series: string; points: number[] }
	| {
			kind: "table";
			title: string;
			columns: string[];
			rows: string[][];
	  };

export interface UseCase {
	id: UseCaseId;
	label: string;
	/** Example text shown as a suggestion chip in the "what are you building" box. */
	example: string;
	/** Keywords used to classify free text. */
	keywords: string[];
	actors: RecommendedActor[];
	improvements: Improvement[];
	dashboard: {
		title: string;
		/** Data the generated dashboard reads, used to describe it. */
		scopes: string[];
		widgets: DashboardWidget[];
	};
}

const COMMON_IMPROVEMENTS: Improvement[] = [
	{
		area: "security",
		title: "Lock down the IP allowlist",
		description:
			"The cluster currently accepts connections from 0.0.0.0/0. Restrict it to your app servers and CI.",
		prompt: "Replace the 0.0.0.0/0 entry in this Rivet cluster's IP allowlist with the CIDR ranges of my application servers and CI runners. Ask me for the ranges if you cannot detect them from my deployment config, then verify a connection from an allowed range still works.",
	},
	{
		area: "observability",
		title: "Wire up alerting",
		description:
			"Nobody is paged when an actor crash-loops. Send cluster alerts to Slack or PagerDuty.",
		prompt: "Set up alerting for this Rivet cluster: actor crash loops, actor node pool saturation above 80%, and failed deploys. Ask me whether to send alerts to Slack or PagerDuty.",
	},
	{
		area: "scale",
		title: "Enable actor node autoscaling",
		description:
			"The default pool is fixed at 4 actor nodes. Scale between 2 and 16 on actor demand.",
		prompt: "Configure the default actor node pool on this Rivet cluster to autoscale between 2 and 16 nodes based on active actor count, and add a second pool for background jobs so they cannot starve interactive actors.",
	},
	{
		area: "reliability",
		product: "Workflows",
		title: "Turn multi-step jobs into Workflows",
		description:
			"Anything that calls more than one external API should replay from the last step instead of starting over.",
		prompt: "Find the multi-step operations in this app (anything that chains external API calls, retries, or waits) and rewrite each as a Rivet Workflow so a crash resumes from the last completed step. Keep the existing actor actions as the entry points, run it locally with `rivet dev`, then deploy to this cluster.",
	},
];

export const USE_CASES: UseCase[] = [
	{
		id: "chat",
		label: "Realtime chat",
		example: "A Slack-style chat app with channels, DMs, and presence",
		keywords: [
			"chat",
			"message",
			"channel",
			"dm",
			"presence",
			"slack",
			"discord",
		],
		actors: [
			{
				name: "ChatRoom",
				key: "room:{roomId}",
				description:
					"One actor per channel. Holds the message log, broadcasts new messages to connected clients.",
			},
			{
				name: "UserPresence",
				key: "user:{userId}",
				description:
					"Tracks online/typing state per user and fans it out to the rooms they are in.",
			},
			{
				name: "Inbox",
				key: "inbox:{userId}",
				description:
					"Per-user unread counts and mention notifications, updated by ChatRoom events.",
			},
			{
				name: "RateLimiter",
				key: "ratelimit:{userId}",
				description:
					"Sliding-window limiter in front of message sends so one client cannot flood a room.",
			},
		],
		improvements: [
			{
				area: "product",
				title: "Add message search",
				description:
					"Index ChatRoom history into a per-room SQLite FTS table so search stays local to the actor.",
				prompt: "Add full-text search to the ChatRoom actor using the actor's SQLite database with an FTS5 table, expose a `search(query)` action, and add a search box to the generated dashboard that calls it.",
			},
			{
				area: "product",
				product: "Workflows",
				title: "Run moderation as a Workflow",
				description:
					"Flag, classify with a model, notify a moderator, and act. Each step is durable, so a restart never drops a report.",
				prompt: "Add a Rivet Workflow for message moderation on this cluster: ChatRoom reports a flagged message, the workflow classifies it with an LLM, waits for a moderator decision with a 24h timeout, then deletes the message or clears the flag and notifies the reporter. Show pending reports in the Chat operations dashboard.",
			},
			{
				area: "security",
				product: "Secure Exec",
				title: "Sandbox custom bots and slash commands",
				description:
					"Let workspaces ship their own bots without giving their code access to other rooms or the admin token.",
				prompt: "Add a Bot actor that runs workspace-authored JavaScript with Rivet Secure Exec, exposing only a `reply(roomId, text)` capability and a 50ms CPU budget per invocation. Wire ChatRoom to invoke bots on slash commands, and add a bots table to the dashboard.",
			},
			...COMMON_IMPROVEMENTS,
		],
		dashboard: {
			title: "Chat operations",
			scopes: [
				"actors:read",
				"actors:ChatRoom:actions:stats",
				"metrics:read",
			],
			widgets: [
				{
					kind: "stat",
					title: "Active rooms",
					value: "1,284",
					delta: "+6% today",
				},
				{
					kind: "stat",
					title: "Connected users",
					value: "9,412",
					delta: "+1,102 vs. 1h ago",
				},
				{ kind: "stat", title: "Messages / min", value: "4,870" },
				{ kind: "stat", title: "p95 send latency", value: "38 ms" },
				{
					kind: "chart",
					title: "Messages per minute",
					series: "messages",
					points: [
						3200, 3500, 3900, 4400, 4100, 4800, 5200, 4900, 4700,
						4870,
					],
				},
				{
					kind: "table",
					title: "Busiest rooms",
					columns: ["Room", "Users", "Msg/min", "Node"],
					rows: [
						["room:general", "2,104", "1,240", "default-2"],
						["room:random", "1,588", "812", "default-1"],
						["room:incidents", "412", "640", "default-3"],
						["room:design", "230", "95", "default-2"],
					],
				},
			],
		},
	},
	{
		id: "ai-agent",
		label: "AI agent",
		example:
			"A coding agent that runs long tasks with tools and streams results",
		keywords: [
			"agent",
			"llm",
			"ai",
			"tool",
			"assistant",
			"openai",
			"anthropic",
			"claude",
			"gpt",
		],
		actors: [
			{
				name: "AgentSession",
				key: "session:{sessionId}",
				description:
					"One actor per conversation. Owns the message history, streams tokens, and survives client disconnects.",
			},
			{
				name: "ToolRunner",
				key: "tool:{sessionId}:{callId}",
				description:
					"Executes a single tool call with retries and a timeout, then reports back to the session.",
			},
			{
				name: "AgentWorkflow",
				key: "workflow:{taskId}",
				description:
					"Durable multi-step task runner so a long job resumes after a crash instead of restarting.",
			},
			{
				name: "UsageMeter",
				key: "usage:{orgId}",
				description:
					"Token and cost accounting per customer, used for billing and spend limits.",
			},
		],
		improvements: [
			{
				area: "product",
				product: "agentOS",
				title: "Give each agent its own sandbox",
				description:
					"Mount an agentOS sandbox into AgentSession so tool calls run in an isolated VM with a filesystem, shell, and network.",
				prompt: "Add agentOS sandbox mounting to the AgentSession actor so each session gets an isolated VM for shell and file tools. Start locally with `rivet dev`, verify a tool call executes inside the sandbox, then deploy to the cluster.",
			},
			{
				area: "reliability",
				product: "Workflows",
				title: "Make long tool chains a Workflow",
				description:
					"Research and multi-hour tasks should resume from the last tool result after a node restart, not replay the whole conversation.",
				prompt: "Refactor the AgentSession run loop so each agent task is a Rivet Workflow: every model call and tool call is a durable step, long waits (human approval, scheduled follow-ups) use workflow sleeps, and a restarted node resumes from the last completed step. Show step progress per session in the Agent fleet dashboard.",
			},
			{
				area: "security",
				product: "Secure Exec",
				title: "Run model-written code in Secure Exec",
				description:
					"Small snippets the model writes (parsers, calculators, data transforms) do not need a full VM. Run them in-process with no I/O.",
				prompt: "Add a `runCode` tool to the AgentSession actor that executes model-generated JavaScript with Rivet Secure Exec: no filesystem or network, a 200ms CPU limit, and only the tool's input available in scope. Fall back to the agentOS sandbox when the code needs shell or network access.",
			},
			...COMMON_IMPROVEMENTS,
		],
		dashboard: {
			title: "Agent fleet",
			scopes: [
				"actors:read",
				"actors:AgentSession:actions:stats",
				"metrics:read",
			],
			widgets: [
				{
					kind: "stat",
					title: "Running sessions",
					value: "312",
					delta: "+24 last hour",
				},
				{ kind: "stat", title: "Tool calls / min", value: "1,906" },
				{
					kind: "stat",
					title: "Tokens today",
					value: "48.2M",
					delta: "$96.40",
				},
				{ kind: "stat", title: "Failed tool calls", value: "0.4%" },
				{
					kind: "chart",
					title: "Tokens per minute",
					series: "tokens",
					points: [
						21000, 24000, 30000, 28000, 35000, 41000, 39000, 44000,
						42000, 46000,
					],
				},
				{
					kind: "table",
					title: "Longest running sessions",
					columns: ["Session", "Customer", "Duration", "Tool calls"],
					rows: [
						["session:8f3a…", "acme", "2h 14m", "1,204"],
						["session:11c0…", "globex", "1h 52m", "880"],
						["session:9d77…", "initech", "58m", "412"],
						["session:02be…", "acme", "41m", "301"],
					],
				},
			],
		},
	},
	{
		id: "multiplayer-game",
		label: "Multiplayer game",
		example:
			"A browser .io game with lobbies, matchmaking, and leaderboards",
		keywords: [
			"game",
			"lobby",
			"match",
			"player",
			"leaderboard",
			"multiplayer",
			"server",
		],
		actors: [
			{
				name: "GameRoom",
				key: "match:{matchId}",
				description:
					"Authoritative game loop at a fixed tick rate. Receives inputs, broadcasts state deltas.",
			},
			{
				name: "Matchmaker",
				key: "matchmaker:{region}",
				description:
					"Queues players by skill and region and spawns a GameRoom when a match is formed.",
			},
			{
				name: "Leaderboard",
				key: "leaderboard:{season}",
				description:
					"Sorted set of scores per season, updated when a GameRoom ends.",
			},
			{
				name: "PlayerProfile",
				key: "player:{playerId}",
				description:
					"Durable inventory, rating, and session token for one player.",
			},
		],
		improvements: [
			{
				area: "scale",
				title: "Pin game rooms to the nearest region",
				description:
					"Spawn GameRoom actors in the region closest to the majority of players in the match.",
				prompt: "Update the Matchmaker actor to spawn GameRoom actors in the actor node pool region closest to the players in the match. Add eu-west-1 and ap-northeast-1 actor node pools to this Rivet cluster and show the latency improvement in the dashboard.",
			},
			{
				area: "product",
				product: "Workflows",
				title: "Run tournaments as a Workflow",
				description:
					"Brackets, round timers, and prize payouts span hours and several services. A Workflow survives restarts mid-round.",
				prompt: "Add a Tournament Rivet Workflow to this app: it creates GameRoom actors per round, waits for results with a per-round deadline, advances the bracket, and pays out through the payments API as a durable step. Show live brackets in the Live ops dashboard.",
			},
			{
				area: "security",
				product: "Secure Exec",
				title: "Sandbox player-made mods",
				description:
					"Let players script custom game modes without letting their code touch other rooms or the host.",
				prompt: "Add mod support to the GameRoom actor using Rivet Secure Exec: load player-authored JavaScript rules with a fixed capability set (read state, emit events), a per-tick CPU budget, and no I/O. Add a mods table with per-mod CPU usage to the dashboard.",
			},
			...COMMON_IMPROVEMENTS,
		],
		dashboard: {
			title: "Live ops",
			scopes: [
				"actors:read",
				"actors:GameRoom:actions:stats",
				"actors:Leaderboard:actions:top",
				"metrics:read",
			],
			widgets: [
				{
					kind: "stat",
					title: "Players online",
					value: "18,420",
					delta: "peak 22,100",
				},
				{ kind: "stat", title: "Active matches", value: "1,906" },
				{ kind: "stat", title: "Avg. queue time", value: "6.2 s" },
				{ kind: "stat", title: "Tick p99", value: "11 ms" },
				{
					kind: "chart",
					title: "Players online",
					series: "players",
					points: [
						9000, 10400, 12800, 15200, 17100, 18900, 21500, 22100,
						19800, 18420,
					],
				},
				{
					kind: "table",
					title: "Regions",
					columns: ["Region", "Matches", "Players", "p99 tick"],
					rows: [
						["us-east-1", "1,012", "9,840", "9 ms"],
						["eu-west-1", "604", "6,120", "12 ms"],
						["ap-northeast-1", "290", "2,460", "14 ms"],
					],
				},
			],
		},
	},
	{
		id: "collab-docs",
		label: "Collaborative docs",
		example: "A Notion-like editor with live cursors and comments",
		keywords: [
			"doc",
			"document",
			"editor",
			"collab",
			"crdt",
			"yjs",
			"cursor",
			"notion",
			"whiteboard",
		],
		actors: [
			{
				name: "Document",
				key: "doc:{docId}",
				description:
					"Holds the CRDT state for one document, merges client updates, and persists snapshots.",
			},
			{
				name: "Awareness",
				key: "doc:{docId}:awareness",
				description:
					"Ephemeral cursors and selections, broadcast at high frequency without touching storage.",
			},
			{
				name: "CommentThread",
				key: "thread:{threadId}",
				description:
					"Comments anchored to a document range, with resolve/reopen history.",
			},
			{
				name: "Workspace",
				key: "workspace:{workspaceId}",
				description:
					"Membership, permissions, and the document tree for one team.",
			},
		],
		improvements: [
			{
				area: "reliability",
				title: "Snapshot documents hourly",
				description:
					"Compact the CRDT update log into a snapshot so cold loads stay fast as documents age.",
				prompt: "Add an hourly schedule to the Document actor that compacts the CRDT update log into a snapshot stored in the actor's SQLite database, and expose a `history()` action so the dashboard can show version counts.",
			},
			{
				area: "product",
				product: "Workflows",
				title: "Publish and export as a Workflow",
				description:
					"Render to PDF, upload to object storage, notify collaborators. Durable steps mean a failed upload retries instead of re-rendering.",
				prompt: "Add a Publish Rivet Workflow for the Document actor: snapshot the document, render it to PDF, upload the file to this cluster's object storage bucket, and email collaborators the link. Each step must be durable and retried independently. Show export history in the Editor health dashboard.",
			},
			{
				area: "security",
				product: "Secure Exec",
				title: "Run user formulas in Secure Exec",
				description:
					"Spreadsheet-style formulas and plugins written by users run in-process, with no access to other documents.",
				prompt: "Add a formula engine to the Document actor using Rivet Secure Exec: evaluate user-authored expressions with only the current document's cells in scope, a 20ms CPU limit per evaluation, and no I/O. Surface slow or failing formulas per document in the dashboard.",
			},
			{
				area: "product",
				product: "Dynamic Apps",
				title: "Give every workspace its own backend",
				description:
					"Workspaces that want custom integrations get a generated backend deployed the moment it is created.",
				prompt: "Add a Dynamic App per workspace to this cluster so a workspace admin can describe a custom integration (webhooks, sync to their CRM) and get a generated backend deployed next to their Document actors. Scope each app's token to that workspace's documents only.",
			},
			...COMMON_IMPROVEMENTS,
		],
		dashboard: {
			title: "Editor health",
			scopes: [
				"actors:read",
				"actors:Document:actions:stats",
				"metrics:read",
			],
			widgets: [
				{
					kind: "stat",
					title: "Open documents",
					value: "3,208",
					delta: "+310 today",
				},
				{ kind: "stat", title: "Editing now", value: "1,142" },
				{ kind: "stat", title: "Updates / sec", value: "2,640" },
				{ kind: "stat", title: "Snapshot lag", value: "12 min" },
				{
					kind: "chart",
					title: "CRDT updates per second",
					series: "updates",
					points: [
						1800, 1950, 2100, 2400, 2300, 2650, 2900, 2800, 2700,
						2640,
					],
				},
				{
					kind: "table",
					title: "Largest documents",
					columns: ["Document", "Editors", "Size", "Snapshots"],
					rows: [
						["doc:roadmap-2026", "48", "14.2 MB", "212"],
						["doc:eng-handbook", "31", "9.8 MB", "188"],
						["doc:design-system", "22", "6.1 MB", "97"],
					],
				},
			],
		},
	},
	{
		id: "custom",
		label: "Something else",
		example: "Describe what you are building",
		keywords: [],
		actors: [
			{
				name: "Session",
				key: "session:{id}",
				description:
					"Per-user or per-connection state that outlives a single request.",
			},
			{
				name: "Workflow",
				key: "workflow:{id}",
				description:
					"A durable multi-step operation that resumes after restarts.",
			},
			{
				name: "RateLimiter",
				key: "ratelimit:{key}",
				description:
					"Sliding-window limiter you can put in front of any action.",
			},
		],
		improvements: COMMON_IMPROVEMENTS,
		dashboard: {
			title: "Cluster overview",
			scopes: ["actors:read", "metrics:read"],
			widgets: [
				{ kind: "stat", title: "Active actors", value: "2,410" },
				{ kind: "stat", title: "Requests / min", value: "38,200" },
				{ kind: "stat", title: "p95 latency", value: "42 ms" },
				{ kind: "stat", title: "Errors", value: "0.1%" },
				{
					kind: "chart",
					title: "Requests per minute",
					series: "requests",
					points: [
						21000, 24000, 27000, 31000, 29000, 33000, 36000, 35000,
						37000, 38200,
					],
				},
				{
					kind: "table",
					title: "Actors by name",
					columns: ["Actor", "Instances", "Req/min", "Pool"],
					rows: [
						["Session", "1,840", "29,100", "default"],
						["Workflow", "412", "6,400", "default"],
						["RateLimiter", "158", "2,700", "default"],
					],
				},
			],
		},
	},
];

/** Coding agents shown in the "connect your agent" bar. */
export type AgentId = "claude-code" | "codex" | "cursor" | "opencode";

export interface AgentClient {
	id: AgentId;
	label: string;
	/** Shell command that installs the Rivet MCP server for this client. */
	mcpInstall: (mcpUrl: string) => string;
}

export const AGENT_CLIENTS: AgentClient[] = [
	{
		id: "claude-code",
		label: "Claude Code",
		mcpInstall: (url) => `claude mcp add --transport http rivet "${url}"`,
	},
	{
		id: "codex",
		label: "Codex",
		mcpInstall: (url) => `codex mcp add rivet --url ${url}`,
	},
	{
		id: "cursor",
		label: "Cursor",
		mcpInstall: (url) =>
			`# .cursor/mcp.json\n${JSON.stringify({ mcpServers: { rivet: { url } } }, null, 2)}`,
	},
	{
		id: "opencode",
		label: "OpenCode",
		mcpInstall: (url) =>
			`# ~/.config/opencode/opencode.jsonc, under "mcp"\n"rivet": { "type": "remote", "url": "${url}", "enabled": true, "oauth": {} }`,
	},
];

/** Fixture rows for the builds table: every build pushed to the cluster. */
export interface Build {
	id: string;
	version: string;
	commit: string;
	message: string;
	createdAgo: string;
}

export const BUILDS: Build[] = [
	{
		id: "b_12",
		version: "v12",
		commit: "a81f2c3",
		message: "feat: add RateLimiter in front of ChatRoom.send",
		createdAgo: "14m ago",
	},
	{
		id: "b_11",
		version: "v11",
		commit: "7c0d9e1",
		message: "fix: persist presence on reconnect",
		createdAgo: "3h ago",
	},
	{
		id: "b_10",
		version: "v10",
		commit: "f31ab77",
		message: "chore: bump rivetkit",
		createdAgo: "1d ago",
	},
	{
		id: "b_09",
		version: "v9",
		commit: "2bd41e0",
		message: "feat: inbox unread counts",
		createdAgo: "1d ago",
	},
];
