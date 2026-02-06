export type DocLink = {
	label: string;
	href: string;
};

export type ActionTemplate = {
	label: string;
	action: string;
	args: unknown[];
	description?: string;
};

export type DemoType = "actions" | "config" | "raw-http" | "raw-websocket";

export type PageConfig = {
	id: string;
	title: string;
	description: string;
	docs: DocLink[];
	actors: string[];
	snippet: string;
	demo?: DemoType;
	rawHttpDefaults?: {
		path: string;
		method: string;
		body?: string;
	};
};

export type PageGroup = {
	id: string;
	title: string;
	pages: PageConfig[];
};

const SNIPPETS = {
	registry: `import { setup } from "rivetkit";

export const registry = setup({
	use: { counter, counterConn },
});`,
	actions: `const actor = client.counter.getOrCreate(["demo"]);
await actor.increment(1);
const value = await actor.getCount();`,
	state: `export const counter = actor({
	state: { count: 0 },
	actions: {
		increment: (c) => ++c.state.count,
	},
});`,
	events: `c.broadcast("newCount", c.state.count);
actor.useEvent("newCount", setCount);`,
	params: `const actor = useActor({
	name: "counterWithParams",
	key: ["demo"],
	params: { region: "us-east" },
});`,
	metadata: `const metadata = await actor.getMetadata();
console.log(metadata.tags, metadata.region);`,
	vars: `export const dynamicVarActor = actor({
	createVars: () => ({ random: Math.random() }),
	actions: { getVars: (c) => c.vars },
});`,
	kv: `await actor.putText("greeting", "hello");
const value = await actor.getText("greeting");`,
	queue: `await actor.queue.work.send({ id: "task-1" });
const message = await actor.receiveOne("work");`,
	workflow: `const workflow = client.order.getOrCreate([orderId]);
await workflow.getOrder();`,
	rawHttp: `const response = await actor.fetch("/api/hello");
const data = await response.json();`,
	rawWebSocket: `const socket = actor.webSocket("/chat");
socket.send(JSON.stringify({ type: "ping" }));`,
	connections: `actor.useEvent("userConnected", (event) => {
	console.log(event.id);
});`,
	lifecycle: `export const counterWithLifecycle = actor({
	onConnect: (c) => c.state.events.push("onConnect"),
	actions: { getEvents: (c) => c.state.events },
});`,
	destroy: `await client.destroyActor.getOrCreate(["demo"]).destroy();`,
	testing: `const { client } = await setupTest(ctx, registry);
const actor = client.counter.getOrCreate(["demo"]);`,
	ai: `const reply = await actor.sendMessage("Hello AI");
actor.useEvent("messageReceived", console.log);`,
};

export const ACTION_TEMPLATES: Record<string, ActionTemplate[]> = {
	counter: [
		{ label: "Increment", action: "increment", args: [1] },
		{ label: "Set Count", action: "setCount", args: [10] },
		{ label: "Get Count", action: "getCount", args: [] },
	],
	counterConn: [
		{ label: "Increment", action: "increment", args: [1] },
		{ label: "Get Count", action: "getCount", args: [] },
		{ label: "Connections", action: "getConnectionCount", args: [] },
	],
	inputActor: [{ label: "Get Inputs", action: "getInputs", args: [] }],
	syncActionActor: [
		{ label: "Increment", action: "increment", args: [1] },
		{ label: "Get Info", action: "getInfo", args: [] },
	],
	asyncActionActor: [
		{ label: "Delayed", action: "delayedIncrement", args: [1] },
		{ label: "Fetch Data", action: "fetchData", args: ["demo"] },
	],
	promiseActor: [
		{ label: "Resolved", action: "resolvedPromise", args: [] },
		{ label: "Delayed", action: "delayedPromise", args: [] },
	],
	shortTimeoutActor: [
		{ label: "Quick", action: "quickAction", args: [] },
		{ label: "Slow", action: "slowAction", args: [] },
	],
	longTimeoutActor: [{ label: "Delayed", action: "delayedAction", args: [] }],
	defaultTimeoutActor: [
		{ label: "Normal", action: "normalAction", args: [] },
	],
	syncTimeoutActor: [
		{ label: "Sync Action", action: "syncAction", args: [] },
	],
	customTimeoutActor: [
		{ label: "Quick", action: "quickAction", args: [] },
		{ label: "Slow", action: "slowAction", args: [] },
	],
	errorHandlingActor: [
		{ label: "Throw Simple", action: "throwSimpleError", args: [] },
		{ label: "Throw Detailed", action: "throwDetailedError", args: [] },
		{ label: "Success", action: "successfulAction", args: [] },
	],
	onStateChangeActor: [
		{ label: "Set Value", action: "setValue", args: [5] },
		{ label: "Get Value", action: "getValue", args: [] },
		{ label: "Change Count", action: "getChangeCount", args: [] },
	],
	metadataActor: [
		{ label: "Set Tags", action: "setupTestTags", args: [{ env: "demo" }] },
		{ label: "Get Metadata", action: "getMetadata", args: [] },
	],
	staticVarActor: [{ label: "Get Vars", action: "getVars", args: [] }],
	nestedVarActor: [
		{ label: "Modify Nested", action: "modifyNested", args: [] },
		{ label: "Get Vars", action: "getVars", args: [] },
	],
	dynamicVarActor: [{ label: "Get Vars", action: "getVars", args: [] }],
	uniqueVarActor: [{ label: "Get Vars", action: "getVars", args: [] }],
	driverCtxActor: [{ label: "Get Vars", action: "getVars", args: [] }],
	kvActor: [
		{ label: "Put Text", action: "putText", args: ["greeting", "hello"] },
		{ label: "Get Text", action: "getText", args: ["greeting"] },
	],
	largePayloadActor: [
		{
			label: "Large Response",
			action: "getLargeResponse",
			args: [50],
		},
	],
	largePayloadConnActor: [
		{
			label: "Large Request",
			action: "processLargeRequest",
			args: [{ items: ["a", "b", "c"] }],
		},
		{ label: "Last Size", action: "getLastRequestSize", args: [] },
	],
	connStateActor: [
		{ label: "Conn State", action: "getConnectionState", args: [] },
		{ label: "Shared +1", action: "incrementSharedCounter", args: [1] },
	],
	counterWithParams: [
		{ label: "Increment", action: "increment", args: [1] },
		{ label: "Initializers", action: "getInitializers", args: [] },
	],
	rejectConnectionActor: [{ label: "Ping", action: "ping", args: [] }],
	requestAccessActor: [
		{ label: "Get Request", action: "getRequestInfo", args: [] },
	],
	counterWithLifecycle: [
		{ label: "Get Events", action: "getEvents", args: [] },
		{ label: "Increment", action: "increment", args: [1] },
	],
	runWithTicks: [{ label: "Get State", action: "getState", args: [] }],
	runWithQueueConsumer: [
		{ label: "Get State", action: "getState", args: [] },
	],
	runWithEarlyExit: [{ label: "Get State", action: "getState", args: [] }],
	runWithError: [{ label: "Get State", action: "getState", args: [] }],
	runWithoutHandler: [{ label: "Get State", action: "getState", args: [] }],
	sleep: [
		{ label: "Get Counts", action: "getCounts", args: [] },
		{ label: "Trigger Sleep", action: "triggerSleep", args: [] },
	],
	sleepWithLongRpc: [{ label: "Get Counts", action: "getCounts", args: [] }],
	sleepWithNoSleepOption: [
		{ label: "Get Counts", action: "getCounts", args: [] },
	],
	scheduled: [
		{ label: "Schedule +5s", action: "scheduleTaskAfter", args: [5000] },
		{ label: "Last Run", action: "getLastRun", args: [] },
	],
	destroyActor: [{ label: "Destroy", action: "destroy", args: [] }],
	destroyObserver: [
		{ label: "Was Destroyed", action: "wasDestroyed", args: ["demo"] },
		{ label: "Reset", action: "reset", args: [] },
	],
	hibernationActor: [
		{ label: "Ping", action: "ping", args: [] },
		{ label: "Actor Counts", action: "getActorCounts", args: [] },
	],
	queueActor: [
		{ label: "Receive One", action: "receiveOne", args: ["work"] },
	],
	sender: [{ label: "Get Messages", action: "getMessages", args: [] }],
	multiQueue: [{ label: "Get Messages", action: "getMessages", args: [] }],
	timeout: [{ label: "Wait", action: "waitForMessage", args: [2000] }],
	worker: [{ label: "Get State", action: "getState", args: [] }],
	selfSender: [{ label: "Get State", action: "getState", args: [] }],
	keepAwake: [{ label: "Get State", action: "getState", args: [] }],
	order: [{ label: "Get Order", action: "getOrder", args: [] }],
	timer: [{ label: "Get Timer", action: "getTimer", args: [] }],
	batch: [{ label: "Get Job", action: "getJob", args: [] }],
	approval: [
		{ label: "Get Request", action: "getRequest", args: [] },
		{ label: "Approve", action: "approve", args: ["Casey"] },
	],
	dashboard: [
		{ label: "Get State", action: "getState", args: [] },
		{ label: "Refresh", action: "refresh", args: [] },
	],
	race: [{ label: "Get Task", action: "getTask", args: [] }],
	payment: [{ label: "Get Transaction", action: "getTransaction", args: [] }],
	inventory: [{ label: "Get Stock", action: "getStock", args: [] }],
	checkout: [
		{ label: "Get Summary", action: "getSummary", args: [] },
		{ label: "Complete", action: "completeCheckout", args: [] },
	],
	aiAgent: [
		{ label: "Get Messages", action: "getMessages", args: [] },
		{ label: "Send Message", action: "sendMessage", args: ["Hello AI"] },
	],
};

export const PAGE_GROUPS: PageGroup[] = [
	{
		id: "overview",
		title: "Overview",
		pages: [
			{
				id: "welcome",
				title: "Welcome",
				description:
					"Get oriented with the Actor Sandbox and how each page maps to the Rivet Actor API.",
				docs: [],
				actors: [],
				snippet: SNIPPETS.registry,
				demo: "config",
			},
			{
				id: "registry-keys",
				title: "Registry and Keys",
				description:
					"See how registry setup and actor keys control identity and instance reuse.",
				docs: [
					{
						label: "Actor Keys",
						href: "https://rivet.dev/docs/actors/keys",
					},
					{
						label: "Registry Configuration",
						href: "https://rivet.dev/docs/general/registry-configuration",
					},
				],
				actors: ["counter", "counterConn"],
				snippet: SNIPPETS.registry,
			},
		],
	},
	{
		id: "core-api",
		title: "Core API",
		pages: [
			{
				id: "actor-configuration",
				title: "Actor Configuration",
				description:
					"Explore config fields like timeouts, memory, and runtime options.",
				docs: [
					{
						label: "Actor Configuration",
						href: "https://rivet.dev/docs/general/actor-configuration",
					},
				],
				actors: [],
				snippet: SNIPPETS.lifecycle,
				demo: "config",
			},
			{
				id: "actions",
				title: "Actions",
				description:
					"Invoke sync, async, and promise-based actions with input payloads.",
				docs: [
					{
						label: "Actions",
						href: "https://rivet.dev/docs/actors/actions",
					},
					{
						label: "Input",
						href: "https://rivet.dev/docs/actors/input",
					},
				],
				actors: [
					"inputActor",
					"syncActionActor",
					"asyncActionActor",
					"promiseActor",
				],
				snippet: SNIPPETS.actions,
			},
			{
				id: "types-helper-types",
				title: "Types and Helper Types",
				description:
					"Reference type helpers to keep actor interfaces end-to-end typed.",
				docs: [
					{
						label: "Types",
						href: "https://rivet.dev/docs/actors/types",
					},
					{
						label: "Helper Types",
						href: "https://rivet.dev/docs/actors/helper-types",
					},
				],
				actors: [],
				snippet: SNIPPETS.actions,
				demo: "config",
			},
			{
				id: "action-timeouts",
				title: "Action Timeouts",
				description:
					"Compare per-actor timeouts for fast and slow action execution.",
				docs: [
					{
						label: "Errors",
						href: "https://rivet.dev/docs/actors/errors",
					},
				],
				actors: [
					"shortTimeoutActor",
					"longTimeoutActor",
					"defaultTimeoutActor",
					"syncTimeoutActor",
					"customTimeoutActor",
				],
				snippet: SNIPPETS.actions,
			},
			{
				id: "errors",
				title: "Errors",
				description:
					"Trigger user errors and timeouts to see standardized error payloads.",
				docs: [
					{
						label: "Errors",
						href: "https://rivet.dev/docs/actors/errors",
					},
				],
				actors: ["errorHandlingActor"],
				snippet: SNIPPETS.actions,
			},
			{
				id: "appearance",
				title: "Appearance",
				description:
					"Review actor appearance controls for dashboards and tooling.",
				docs: [
					{
						label: "Appearance",
						href: "https://rivet.dev/docs/actors/appearance",
					},
				],
				actors: [],
				snippet: SNIPPETS.registry,
				demo: "config",
			},
		],
	},
	{
		id: "state-storage",
		title: "State and Storage",
		pages: [
			{
				id: "state-basics",
				title: "State Basics",
				description:
					"Read and mutate actor state while broadcasting updates.",
				docs: [
					{
						label: "State",
						href: "https://rivet.dev/docs/actors/state",
					},
				],
				actors: ["counter"],
				snippet: SNIPPETS.state,
			},
			{
				id: "on-state-change",
				title: "On State Change",
				description:
					"Track how often state changes fire with onStateChange hooks.",
				docs: [
					{
						label: "Events",
						href: "https://rivet.dev/docs/actors/events",
					},
				],
				actors: ["onStateChangeActor"],
				snippet: SNIPPETS.state,
			},
			{
				id: "sharing-joining",
				title: "Sharing and Joining State",
				description:
					"Coordinate shared state across actors and listeners.",
				docs: [
					{
						label: "Sharing and Joining State",
						href: "https://rivet.dev/docs/actors/sharing-and-joining-state",
					},
				],
				actors: [],
				snippet: SNIPPETS.connections,
				demo: "config",
			},
			{
				id: "metadata",
				title: "Metadata",
				description:
					"Inspect actor metadata like tags, regions, and names.",
				docs: [
					{
						label: "Metadata",
						href: "https://rivet.dev/docs/actors/metadata",
					},
				],
				actors: ["metadataActor"],
				snippet: SNIPPETS.metadata,
			},
			{
				id: "ephemeral-variables",
				title: "Ephemeral Variables",
				description:
					"Compare static, nested, and dynamic vars with driver context.",
				docs: [
					{
						label: "Ephemeral Variables",
						href: "https://rivet.dev/docs/actors/ephemeral-variables",
					},
				],
				actors: [
					"staticVarActor",
					"nestedVarActor",
					"dynamicVarActor",
					"uniqueVarActor",
					"driverCtxActor",
				],
				snippet: SNIPPETS.vars,
			},
			{
				id: "kv-storage",
				title: "KV Storage",
				description:
					"Write and read structured values from actor KV storage.",
				docs: [
					{ label: "KV", href: "https://rivet.dev/docs/actors/kv" },
				],
				actors: ["kvActor"],
				snippet: SNIPPETS.kv,
			},
			{
				id: "external-sql",
				title: "External SQL",
				description:
					"Reference external SQL integration patterns for data access.",
				docs: [
					{
						label: "External SQL",
						href: "https://rivet.dev/docs/actors/external-sql",
					},
				],
				actors: [],
				snippet: SNIPPETS.kv,
				demo: "config",
			},
			{
				id: "large-payloads",
				title: "Large Payloads",
				description: "Handle large request and response bodies safely.",
				docs: [],
				actors: ["largePayloadActor", "largePayloadConnActor"],
				snippet: SNIPPETS.actions,
			},
		],
	},
	{
		id: "realtime",
		title: "Realtime and Connections",
		pages: [
			{
				id: "connections-presence",
				title: "Connections and Presence",
				description:
					"Track connection state and shared counters across clients.",
				docs: [
					{
						label: "Connections",
						href: "https://rivet.dev/docs/actors/connections",
					},
				],
				actors: ["connStateActor", "counterWithParams", "counterConn"],
				snippet: SNIPPETS.connections,
			},
			{
				id: "events-broadcasts",
				title: "Events and Broadcasts",
				description:
					"Listen to event broadcasts from actors in realtime.",
				docs: [
					{
						label: "Events",
						href: "https://rivet.dev/docs/actors/events",
					},
				],
				actors: ["connStateActor", "counter"],
				snippet: SNIPPETS.events,
			},
			{
				id: "direct-connection",
				title: "Direct Connection Messaging",
				description: "Send direct messages to specific connection IDs.",
				docs: [
					{
						label: "Connections",
						href: "https://rivet.dev/docs/actors/connections",
					},
				],
				actors: ["connStateActor"],
				snippet: SNIPPETS.connections,
			},
			{
				id: "connection-gating",
				title: "Connection Gating",
				description: "Control which clients can connect to an actor.",
				docs: [
					{
						label: "Authentication",
						href: "https://rivet.dev/docs/actors/authentication",
					},
				],
				actors: ["rejectConnectionActor"],
				snippet: SNIPPETS.connections,
			},
			{
				id: "request-access",
				title: "Request Object Access",
				description: "Inspect raw request details inside handlers.",
				docs: [
					{
						label: "Request Handler",
						href: "https://rivet.dev/docs/actors/request-handler",
					},
					{
						label: "WebSocket Handler",
						href: "https://rivet.dev/docs/actors/websocket-handler",
					},
				],
				actors: ["requestAccessActor"],
				snippet: SNIPPETS.rawHttp,
			},
		],
	},
	{
		id: "http-ws",
		title: "HTTP and WebSocket",
		pages: [
			{
				id: "http-api",
				title: "HTTP API",
				description:
					"Route standard HTTP requests through actor handlers.",
				docs: [
					{
						label: "HTTP API",
						href: "https://rivet.dev/docs/actors/http-api",
					},
				],
				actors: [],
				snippet: SNIPPETS.rawHttp,
				demo: "config",
			},
			{
				id: "request-handler",
				title: "Request Handler",
				description:
					"Use onRequest to serve REST endpoints from an actor.",
				docs: [
					{
						label: "Request Handler",
						href: "https://rivet.dev/docs/actors/request-handler",
					},
				],
				actors: ["rawFetchCounter"],
				snippet: SNIPPETS.rawHttp,
				demo: "raw-http",
				rawHttpDefaults: { path: "/count", method: "GET" },
			},
			{
				id: "websocket-handler",
				title: "WebSocket Handler",
				description: "Handle raw WebSocket connections in an actor.",
				docs: [
					{
						label: "WebSocket Handler",
						href: "https://rivet.dev/docs/actors/websocket-handler",
					},
				],
				actors: ["rawWebSocketChatRoom"],
				snippet: SNIPPETS.rawWebSocket,
				demo: "raw-websocket",
			},
			{
				id: "fetch-websocket-handler",
				title: "Fetch and WebSocket Handler",
				description:
					"Combine fetch and WebSocket handlers on the same actor.",
				docs: [
					{
						label: "Fetch and WebSocket Handler",
						href: "https://rivet.dev/docs/actors/fetch-and-websocket-handler",
					},
				],
				actors: [],
				snippet: SNIPPETS.rawWebSocket,
				demo: "config",
			},
			{
				id: "raw-http",
				title: "Raw HTTP",
				description:
					"Call raw HTTP endpoints on actors without actions.",
				docs: [],
				actors: [
					"rawHttpActor",
					"rawHttpNoHandlerActor",
					"rawHttpVoidReturnActor",
					"rawHttpHonoActor",
					"rawHttpRequestPropertiesActor",
				],
				snippet: SNIPPETS.rawHttp,
				demo: "raw-http",
				rawHttpDefaults: { path: "/api/hello", method: "GET" },
			},
			{
				id: "raw-websocket",
				title: "Raw WebSocket",
				description:
					"Open raw WebSocket connections and exchange messages.",
				docs: [],
				actors: ["rawWebSocketActor", "rawWebSocketBinaryActor"],
				snippet: SNIPPETS.rawWebSocket,
				demo: "raw-websocket",
			},
		],
	},
	{
		id: "lifecycle",
		title: "Lifecycle and Scheduling",
		pages: [
			{
				id: "lifecycle-hooks",
				title: "Lifecycle Hooks",
				description: "Track connect, disconnect, and wake events.",
				docs: [
					{
						label: "Lifecycle",
						href: "https://rivet.dev/docs/actors/lifecycle",
					},
				],
				actors: ["counterWithLifecycle"],
				snippet: SNIPPETS.lifecycle,
			},
			{
				id: "run-handler",
				title: "Run Handler",
				description: "Observe run loops, ticks, and queue consumers.",
				docs: [
					{
						label: "Lifecycle",
						href: "https://rivet.dev/docs/actors/lifecycle",
					},
				],
				actors: [
					"runWithTicks",
					"runWithEarlyExit",
					"runWithError",
					"runWithoutHandler",
					"runWithQueueConsumer",
				],
				snippet: SNIPPETS.lifecycle,
			},
			{
				id: "sleep-wake",
				title: "Sleep and Wake",
				description: "Manage sleep cycles and long-running RPCs.",
				docs: [
					{
						label: "Lifecycle",
						href: "https://rivet.dev/docs/actors/lifecycle",
					},
				],
				actors: ["sleep", "sleepWithLongRpc", "sleepWithNoSleepOption"],
				snippet: SNIPPETS.lifecycle,
			},
			{
				id: "schedule",
				title: "Schedule",
				description: "Schedule alarms and recurring work for actors.",
				docs: [
					{
						label: "Schedule",
						href: "https://rivet.dev/docs/actors/schedule",
					},
				],
				actors: ["scheduled", "sleep"],
				snippet: SNIPPETS.lifecycle,
			},
			{
				id: "destroy",
				title: "Destroy",
				description:
					"Terminate actors and observe destruction callbacks.",
				docs: [
					{
						label: "Destroy",
						href: "https://rivet.dev/docs/actors/destroy",
					},
				],
				actors: ["destroyActor", "destroyObserver"],
				snippet: SNIPPETS.destroy,
			},
			{
				id: "hibernation",
				title: "Hibernation",
				description:
					"Trigger hibernation and inspect state restoration.",
				docs: [],
				actors: ["hibernationActor"],
				snippet: SNIPPETS.lifecycle,
			},
			{
				id: "versions",
				title: "Versions",
				description: "Understand actor versioning for safe rollouts.",
				docs: [
					{
						label: "Versions",
						href: "https://rivet.dev/docs/actors/versions",
					},
				],
				actors: [],
				snippet: SNIPPETS.registry,
				demo: "config",
			},
			{
				id: "scaling",
				title: "Scaling",
				description:
					"Review scaling options for actor capacity and concurrency.",
				docs: [
					{
						label: "Scaling",
						href: "https://rivet.dev/docs/actors/scaling",
					},
				],
				actors: [],
				snippet: SNIPPETS.registry,
				demo: "config",
			},
		],
	},
	{
		id: "queues",
		title: "Queues",
		pages: [
			{
				id: "queue-basics",
				title: "Queue Basics",
				description: "Send and receive queue messages from actors.",
				docs: [
					{
						label: "Queue",
						href: "https://rivet.dev/docs/actors/queue",
					},
				],
				actors: ["queueActor", "queueLimitedActor"],
				snippet: SNIPPETS.queue,
			},
			{
				id: "queue-patterns",
				title: "Queue Patterns",
				description:
					"Explore sending, timeouts, workers, and keep-awake patterns.",
				docs: [
					{
						label: "Queue",
						href: "https://rivet.dev/docs/actors/queue",
					},
				],
				actors: [
					"sender",
					"multiQueue",
					"timeout",
					"worker",
					"selfSender",
					"keepAwake",
				],
				snippet: SNIPPETS.queue,
			},
			{
				id: "queue-run-loop",
				title: "Queue in Run Loop",
				description: "Consume queue messages inside run handlers.",
				docs: [
					{
						label: "Queue",
						href: "https://rivet.dev/docs/actors/queue",
					},
				],
				actors: ["runWithQueueConsumer"],
				snippet: SNIPPETS.queue,
			},
		],
	},
	{
		id: "workflows",
		title: "Workflows",
		pages: [
			{
				id: "workflow-overview",
				title: "Workflow Overview",
				description:
					"Traverse core workflow patterns built on Rivet Actors.",
				docs: [
					{
						label: "Workflows",
						href: "https://rivet.dev/docs/workflows",
					},
				],
				actors: [
					"timer",
					"order",
					"batch",
					"approval",
					"dashboard",
					"race",
					"payment",
				],
				snippet: SNIPPETS.workflow,
			},
			{
				id: "workflow-steps",
				title: "Steps",
				description: "Chain sequential steps with automatic retries.",
				docs: [
					{
						label: "Workflows",
						href: "https://rivet.dev/docs/workflows",
					},
				],
				actors: ["order"],
				snippet: SNIPPETS.workflow,
			},
			{
				id: "workflow-sleep",
				title: "Sleep",
				description: "Suspend workflows with durable timers.",
				docs: [
					{
						label: "Workflows",
						href: "https://rivet.dev/docs/workflows",
					},
				],
				actors: ["timer"],
				snippet: SNIPPETS.workflow,
			},
			{
				id: "workflow-loops",
				title: "Loops",
				description: "Batch tasks with cursor-based loops.",
				docs: [
					{
						label: "Workflows",
						href: "https://rivet.dev/docs/workflows",
					},
				],
				actors: ["batch"],
				snippet: SNIPPETS.workflow,
			},
			{
				id: "workflow-listen",
				title: "Listen",
				description: "Wait for approvals and events in workflows.",
				docs: [
					{
						label: "Workflows",
						href: "https://rivet.dev/docs/workflows",
					},
				],
				actors: ["approval"],
				snippet: SNIPPETS.workflow,
			},
			{
				id: "workflow-join",
				title: "Join",
				description:
					"Aggregate results across concurrent workflow branches.",
				docs: [
					{
						label: "Workflows",
						href: "https://rivet.dev/docs/workflows",
					},
				],
				actors: ["dashboard"],
				snippet: SNIPPETS.workflow,
			},
			{
				id: "workflow-race",
				title: "Race",
				description:
					"Race work against timeouts or alternate branches.",
				docs: [
					{
						label: "Workflows",
						href: "https://rivet.dev/docs/workflows",
					},
				],
				actors: ["race"],
				snippet: SNIPPETS.workflow,
			},
			{
				id: "workflow-rollback",
				title: "Rollback",
				description:
					"Model compensating transactions and rollback logic.",
				docs: [
					{
						label: "Workflows",
						href: "https://rivet.dev/docs/workflows",
					},
				],
				actors: ["payment"],
				snippet: SNIPPETS.workflow,
			},
		],
	},
	{
		id: "inter-actor",
		title: "Inter-Actor and Patterns",
		pages: [
			{
				id: "communicating-between-actors",
				title: "Communicating Between Actors",
				description: "Chain actors together using server-side clients.",
				docs: [
					{
						label: "Communicating Between Actors",
						href: "https://rivet.dev/docs/actors/communicating-between-actors",
					},
				],
				actors: ["inventory", "checkout"],
				snippet: SNIPPETS.actions,
			},
			{
				id: "design-patterns",
				title: "Design Patterns",
				description:
					"Review proven actor design patterns and tradeoffs.",
				docs: [
					{
						label: "Design Patterns",
						href: "https://rivet.dev/docs/actors/design-patterns",
					},
				],
				actors: [],
				snippet: SNIPPETS.registry,
				demo: "config",
			},
		],
	},
	{
		id: "testing",
		title: "Testing and Debugging",
		pages: [
			{
				id: "testing",
				title: "Testing",
				description:
					"Use the driver test suite actors to validate behavior.",
				docs: [
					{
						label: "Testing",
						href: "https://rivet.dev/docs/actors/testing",
					},
				],
				actors: [
					"counter",
					"counterConn",
					"queueActor",
					"rawHttpActor",
					"rawWebSocketActor",
				],
				snippet: SNIPPETS.testing,
			},
		],
	},
	{
		id: "ai-ugc",
		title: "AI and UGC",
		pages: [
			{
				id: "ai-ugc",
				title: "AI and User-Generated Actors",
				description:
					"Combine AI-driven actors with user-generated templates.",
				docs: [
					{
						label: "AI and User-Generated Actors",
						href: "https://rivet.dev/docs/actors/ai-and-user-generated-actors",
					},
				],
				actors: ["aiAgent"],
				snippet: SNIPPETS.ai,
			},
		],
	},
];

export const PAGE_INDEX = PAGE_GROUPS.flatMap((group) =>
	group.pages.map((page) => ({ ...page, group })),
);
