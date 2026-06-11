// Realistic fixtures for the agentOS inspector prototype. Values mirror the
// design mockups exactly so the demo is faithful. The live data hookup (see
// `use-agent-os-inspector.tsx`) replaces these with real `executeAction` calls.
//
// Timestamps are computed relative to load time so `RelativeTime` renders the
// "Xd ago" / "Xs ago" labels from the mockups. This is a fixtures/demo module,
// not production runtime code.

import type {
	AgentOsManifest,
	AgentOsMetadata,
	ConnInfo,
	FileContent,
	FsNode,
	Invocation,
	MountInfo,
	ProcessInfo,
	SessionSummary,
	SoftwareBundle,
	Toolkit,
	TranscriptEvent,
} from "./types";

const NOW = Date.now();
const SEC = 1000;
const MIN = 60 * SEC;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;

const fiveDaysAgo = NOW - 5 * DAY;
const secondsAgo = (s: number) => NOW - s * SEC;

const PRIMARY_SESSION = "ses_0188X9P7K2M3N4Q5R6S7T8";
const SECOND_SESSION = "ses_0188X9N4J1L2M3N4Q5R6S7";

// --- Software (Software tab) ---

export const SOFTWARE: SoftwareBundle[] = [
	{
		name: "@rivet-dev/agent-os-common",
		version: "0.4.2",
		source: "rivet-dev",
		binaries: [
			"sh",
			"bash",
			"ls",
			"cat",
			"cp",
			"mv",
			"rm",
			"grep",
			"sed",
			"awk",
			"find",
			"tar",
			"gzip",
		],
	},
	{
		name: "@rivet-dev/agent-os-pi",
		version: "0.3.1",
		source: "rivet-dev",
		binaries: ["pi"],
	},
	{
		name: "weather-toolkit",
		version: "1.0.0",
		source: "user",
		binaries: ["agentos-weather"],
	},
];

// --- Toolkits + invocations (Tools tab) ---

export const TOOLKITS: Toolkit[] = [
	{
		name: "weather",
		tools: [
			{
				tool: "get",
				command: "agentos-weather get",
				description: "Get the current weather for a city.",
				args: [{ name: "city", type: "string" }],
			},
		],
	},
	{
		name: "review",
		tools: [
			{
				tool: "submit",
				command: "agentos-review submit",
				description: "Submit a file for code review by another agent.",
				args: [{ name: "path", type: "string" }],
			},
		],
	},
	{
		name: "github",
		tools: [
			{
				tool: "open_pr",
				command: "agentos-github open_pr",
				description: "Open a pull request on GitHub.",
				args: [
					{ name: "title", type: "string" },
					{ name: "body", type: "string", optional: true },
					{ name: "branch", type: "string" },
				],
			},
		],
	},
];

export const INVOCATIONS: Invocation[] = [
	{
		tool: "weather.get",
		input: { city: "London" },
		output: {
			city: "London",
			temperature: 18,
			conditions: "partly cloudy",
			humidity: 65,
		},
		latencyMs: 412,
		at: fiveDaysAgo,
	},
	{
		tool: "review.submit",
		input: { path: "/home/user/api/auth.ts" },
		output: "(pending)",
		latencyMs: 0,
		at: fiveDaysAgo,
	},
	{
		tool: "github.open_pr",
		input: { title: "fix(auth): handle 429", branch: "fix-auth-429" },
		output: "https://github.com/rivet-dev/rivet/pull/4842",
		latencyMs: 1204,
		at: fiveDaysAgo,
	},
	{
		tool: "weather.get",
		input: { city: "Atlantis" },
		output: null,
		error: "weather.unknown_city",
		latencyMs: 312,
		at: fiveDaysAgo,
	},
];

// --- Mounts (Mounts tab) ---

export const MOUNTS: MountInfo[] = [
	{
		path: "/",
		kind: "persistent",
		provider: "—",
		sizeBytes: 10 * 1024 * 1024 * 1024,
		status: "online",
	},
	{
		path: "/mnt/uploads",
		kind: "s3",
		provider: "s3://rivet-uploads",
		sizeBytes: Math.round(4.6 * 1024 * 1024),
		status: "online",
	},
	{
		path: "/mnt/sandbox",
		kind: "sandbox",
		provider: "e2b · sjc1",
		status: "online",
	},
	{
		path: "/mnt/share",
		kind: "gdrive",
		provider: "team-drive/agents",
		status: "degraded",
	},
];

// --- Metadata (Metadata tab) ---

export const METADATA: AgentOsMetadata = {
	actorId: "0188X9P7K2M3N4Q5R6S7T8",
	actorKey: "my-agent",
	actorName: "pi-coding-agent",
	actorKind: "agent-os",
	runner: "default · sjc1",
	region: "Northern Virginia, USA",
	agentVersion: "pi 0.3.1",
	agentosCore: "0.4.2",
	software: ["common", "pi", "weather-toolkit"],
	createdAt: Date.parse("2026-05-12T14:21:00Z"),
	sessionCount: 4,
	activeSessionCount: 2,
};

// --- Manifest (detection + static config) ---

export const AGENT_OS_MANIFEST: AgentOsManifest = {
	kind: "agent-os",
	agent: { type: "pi", version: "0.3.1" },
	agentosCore: "0.4.2",
	software: SOFTWARE,
	toolkits: TOOLKITS,
	mounts: MOUNTS,
	metadata: METADATA,
};

// --- Sessions + transcript (Transcript tab) ---

export const SESSIONS: SessionSummary[] = [
	{
		sessionId: PRIMARY_SESSION,
		agentType: "pi",
		createdAt: fiveDaysAgo,
		eventCount: 7,
		status: "running",
	},
	{
		sessionId: SECOND_SESSION,
		agentType: "pi",
		createdAt: fiveDaysAgo,
		eventCount: 23,
		status: "idle",
	},
	{
		sessionId: "ses_0188Q18223R4L5M6N7P8Q9",
		agentType: "pi",
		createdAt: fiveDaysAgo,
		eventCount: 4,
		status: "idle",
	},
	{
		sessionId: "ses_0188E6F7G8H9J0K1L2M3N4",
		agentType: "pi",
		createdAt: fiveDaysAgo,
		eventCount: 11,
		status: "error",
	},
];

const WEATHER_JSON = `{
  "city": "London",
  "temperature": 18,
  "conditions": "partly cloudy",
  "humidity": 65
}`;

export const TRANSCRIPTS: Record<string, TranscriptEvent[]> = {
	[PRIMARY_SESSION]: [
		{
			kind: "user",
			seq: 1,
			at: fiveDaysAgo,
			text: "Write a Python script that fetches the weather in London and saves it to /home/user/weather.json",
		},
		{
			kind: "assistant",
			seq: 2,
			at: fiveDaysAgo,
			text: "I'll first check what's currently on disk, then call the weather tool and write the result.",
		},
		{
			kind: "shell",
			seq: 3,
			at: fiveDaysAgo,
			command: "ls /home/user",
			exitCode: 0,
			durationMs: 14,
			output: "notes.md\nscratch.py",
		},
		{
			kind: "tool",
			seq: 4,
			at: fiveDaysAgo,
			tool: "weather.get",
			input: { city: "London" },
			output: {
				city: "London",
				temperature: 18,
				conditions: "partly cloudy",
				humidity: 65,
			},
			latencyMs: 412,
		},
		{
			kind: "file_write",
			seq: 5,
			at: fiveDaysAgo,
			path: "/home/user/weather.json",
			bytes: 96,
		},
		{
			kind: "shell",
			seq: 6,
			at: fiveDaysAgo,
			command: "cat /home/user/weather.json",
			exitCode: 0,
			durationMs: 8,
			output: WEATHER_JSON,
		},
		{
			kind: "assistant",
			seq: 7,
			at: fiveDaysAgo,
			text: "Saved to /home/user/weather.json. Anything else?",
		},
	],
	// Edge state: a session that hit a non-zero shell exit and a tool error.
	[SECOND_SESSION]: [
		{
			kind: "user",
			seq: 1,
			at: fiveDaysAgo,
			text: "Run the auth test suite and open a PR if it passes.",
		},
		{
			kind: "shell",
			seq: 2,
			at: fiveDaysAgo,
			command: "pnpm test tests/auth",
			exitCode: 1,
			durationMs: 4213,
			output: "✓ tests/auth/login.test.ts (12)\n✓ tests/auth/register.test.ts (8)\n✗ tests/auth/session.test.ts (1) failed\n  AssertionError: expected 401 to equal 403",
		},
		{
			kind: "assistant",
			seq: 3,
			at: fiveDaysAgo,
			text: "One test failed, so I won't open a PR yet. The session expiry check returns 401 where 403 is expected.",
		},
	],
	ses_0188Q18223R4L5M6N7P8Q9: [],
	ses_0188E6F7G8H9J0K1L2M3N4: [
		{
			kind: "user",
			seq: 1,
			at: fiveDaysAgo,
			text: "Deploy the staging environment.",
		},
		{
			kind: "tool",
			seq: 2,
			at: fiveDaysAgo,
			tool: "github.open_pr",
			input: { title: "chore: bump staging", branch: "staging" },
			output: null,
			latencyMs: 880,
			error: "github.permission_denied",
		},
	],
};

/** The session shown selected by default in the Transcript tab. */
export const DEFAULT_SESSION_ID = PRIMARY_SESSION;

// --- Filesystem (Filesystem tab) ---

export const FILESYSTEM: FsNode = {
	name: "/",
	path: "/",
	type: "dir",
	mount: "persistent",
	children: [
		{
			name: "home",
			path: "/home",
			type: "dir",
			mount: "persistent",
			children: [
				{
					name: "user",
					path: "/home/user",
					type: "dir",
					mount: "persistent",
					children: [
						{
							name: "notes.md",
							path: "/home/user/notes.md",
							type: "file",
							mount: "persistent",
							sizeBytes: 1280,
							mtimeMs: fiveDaysAgo,
						},
						{
							name: "scratch.py",
							path: "/home/user/scratch.py",
							type: "file",
							mount: "persistent",
							sizeBytes: 642,
							mtimeMs: fiveDaysAgo,
						},
						{
							name: "weather.json",
							path: "/home/user/weather.json",
							type: "file",
							mount: "persistent",
							sizeBytes: 96,
							mtimeMs: fiveDaysAgo,
						},
					],
				},
			],
		},
		{
			name: "mnt",
			path: "/mnt",
			type: "dir",
			mount: "persistent",
			children: [
				{
					name: "sandbox",
					path: "/mnt/sandbox",
					type: "dir",
					mount: "sandbox",
					children: [
						{
							name: "playwright-trace.zip",
							path: "/mnt/sandbox/playwright-trace.zip",
							type: "file",
							mount: "sandbox",
							sizeBytes: 5_400_000,
							mtimeMs: secondsAgo(120),
						},
					],
				},
				{
					name: "uploads",
					path: "/mnt/uploads",
					type: "dir",
					mount: "s3",
					children: [
						{
							name: "dataset.csv",
							path: "/mnt/uploads/dataset.csv",
							type: "file",
							mount: "s3",
							sizeBytes: 4_600_000,
							mtimeMs: fiveDaysAgo,
						},
					],
				},
			],
		},
		{
			name: "usr",
			path: "/usr",
			type: "dir",
			mount: "persistent",
			children: [
				{
					name: "bin",
					path: "/usr/bin",
					type: "dir",
					mount: "persistent",
				},
			],
		},
	],
};

export const FILE_CONTENTS: Record<string, FileContent> = {
	"/home/user/weather.json": {
		path: "/home/user/weather.json",
		sizeBytes: 96,
		mtimeMs: fiveDaysAgo,
		text: WEATHER_JSON,
		language: "json",
	},
	"/home/user/notes.md": {
		path: "/home/user/notes.md",
		sizeBytes: 1280,
		mtimeMs: fiveDaysAgo,
		text: "# Notes\n\n- Fetch weather for London\n- Save result to weather.json\n- Open a PR when tests pass\n",
		language: "markdown",
	},
	"/home/user/scratch.py": {
		path: "/home/user/scratch.py",
		sizeBytes: 642,
		mtimeMs: fiveDaysAgo,
		text: 'import json\nimport urllib.request\n\n\ndef fetch_weather(city: str) -> dict:\n    url = f"https://api.weather.example/v1/current?city={city}"\n    with urllib.request.urlopen(url) as resp:\n        return json.load(resp)\n\n\nif __name__ == "__main__":\n    data = fetch_weather("London")\n    with open("/home/user/weather.json", "w") as f:\n        json.dump(data, f, indent=2)\n',
		language: "python",
	},
	// Edge state: a binary file the viewer cannot render as text.
	"/mnt/sandbox/playwright-trace.zip": {
		path: "/mnt/sandbox/playwright-trace.zip",
		sizeBytes: 5_400_000,
		mtimeMs: secondsAgo(120),
		text: null,
	},
	"/mnt/uploads/dataset.csv": {
		path: "/mnt/uploads/dataset.csv",
		sizeBytes: 4_600_000,
		mtimeMs: fiveDaysAgo,
		text: "id,name,score\n1,alpha,0.92\n2,beta,0.81\n3,gamma,0.74\n…",
		language: "text",
	},
};

/** File shown selected by default in the Filesystem tab. */
export const DEFAULT_FILE_PATH = "/home/user/weather.json";

// --- Processes (Processes tab) ---

export const PROCESSES: ProcessInfo[] = [
	{
		pid: 1,
		ppid: 0,
		command: "/sbin/init",
		startedAt: fiveDaysAgo,
		cpu: 0.1,
		memBytes: 4 * 1024 * 1024,
		status: "sleeping",
	},
	{
		pid: 14,
		ppid: 1,
		command: `pi --session ${PRIMARY_SESSION}`,
		startedAt: fiveDaysAgo,
		cpu: 8.3,
		memBytes: 142 * 1024 * 1024,
		status: "running",
	},
	{
		pid: 18,
		ppid: 14,
		command: "agentos-weather get --city London",
		startedAt: fiveDaysAgo,
		cpu: 0.0,
		memBytes: 6 * 1024 * 1024,
		status: "sleeping",
	},
	{
		pid: 22,
		ppid: 1,
		command: `pi --session ${SECOND_SESSION}`,
		startedAt: fiveDaysAgo,
		cpu: 1.4,
		memBytes: 138 * 1024 * 1024,
		status: "running",
	},
	{
		pid: 27,
		ppid: 22,
		command: "pnpm test",
		startedAt: fiveDaysAgo,
		cpu: 12.1,
		memBytes: 412 * 1024 * 1024,
		status: "running",
		signal: "SIGTERM",
		stdoutTail:
			"✓ tests/auth/login.test.ts (12)\n✓ tests/auth/register.test.ts (8)\n✗ tests/auth/session.test.ts (1) failed\n  AssertionError: expected 401 to equal 403",
	},
];

/** Process shown selected by default in the Processes tab. */
export const DEFAULT_PID = 27;

// --- Connections (Connections tab) ---

export const CONNECTIONS: ConnInfo[] = [
	{
		connId: "c_RQK7M2N1",
		stream: `sessionEvent · ${PRIMARY_SESSION}`,
		connectedAt: secondsAgo(612),
		origin: "browser",
	},
	{
		connId: "c_M2F8N7K4",
		stream: `sessionEvent · ${PRIMARY_SESSION}`,
		connectedAt: secondsAgo(92),
		origin: "browser",
	},
	{
		connId: "c_T7842ZL1",
		stream: "shell · pid 27",
		connectedAt: secondsAgo(5),
		origin: "cli",
	},
];
