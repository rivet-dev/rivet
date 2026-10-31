import type { Rivet } from "@rivet-gg/cloud";
import type { ActorStatus } from "@/components/actors/queries";
import type { RecommendedActor, UseCase } from "./catalog";

export interface MockActor {
	id: string;
	name: string;
	key: string;
	status: ActorStatus;
	region: string;
	/** Unix ms. */
	createdAt: number;
}

/** Deterministic xorshift PRNG so the mock renders the same on every load. */
export function rng(seed: number) {
	let s = seed >>> 0 || 1;
	return () => {
		s ^= s << 13;
		s ^= s >>> 17;
		s ^= s << 5;
		return (s >>> 0) / 0xffffffff;
	};
}

export function hash(input: string) {
	let h = 2166136261;
	for (let i = 0; i < input.length; i++) {
		h ^= input.charCodeAt(i);
		h = Math.imul(h, 16777619);
	}
	return h >>> 0;
}

const HEX = "0123456789abcdef";

export function hexId(next: () => number, length: number) {
	let out = "";
	for (let i = 0; i < length; i++) {
		out += HEX[Math.floor(next() * 16)];
	}
	return out;
}

const STATUS_WEIGHTS: Array<[ActorStatus, number]> = [
	["running", 0.62],
	["sleeping", 0.3],
	["starting", 0.04],
	["crashed", 0.04],
];

function pickStatus(next: () => number): ActorStatus {
	const r = next();
	let acc = 0;
	for (const [status, weight] of STATUS_WEIGHTS) {
		acc += weight;
		if (r < acc) return status;
	}
	return "running";
}

const KEY_WORDS = [
	"general",
	"design",
	"ops",
	"random",
	"support",
	"launch",
	"eu-sales",
	"incident-42",
];

function fillKey(pattern: string, next: () => number, index: number) {
	return pattern.replace(/\{(\w+)\}/g, (_, param: string) => {
		if (/id$/i.test(param) && /room|doc|match|session|conv/i.test(param)) {
			return KEY_WORDS[index % KEY_WORDS.length];
		}
		return hexId(next, 8);
	});
}

/**
 * Builds a stable list of actor instances for the use case's recommended
 * actors. The first actor type gets the most instances because it is the one
 * users look at first.
 */
export function mockActors(
	useCase: UseCase,
	region: string,
	now = Date.now(),
): MockActor[] {
	const next = rng(hash(`${useCase.id}:${region}`));
	const actors: MockActor[] = [];
	useCase.actors.forEach((actor: RecommendedActor, typeIndex) => {
		const count = typeIndex === 0 ? 6 : 3;
		for (let i = 0; i < count; i++) {
			actors.push({
				id: `${hexId(next, 8)}-${hexId(next, 4)}-4${hexId(next, 3)}-${hexId(next, 4)}-${hexId(next, 12)}`,
				name: actor.name,
				key: fillKey(actor.key, next, typeIndex * 7 + i),
				status: pickStatus(next),
				region,
				createdAt: now - Math.floor(next() * 36 * 60 * 60 * 1000),
			});
		}
	});
	return actors.sort((a, b) => b.createdAt - a.createdAt);
}

export type LogEntry = Rivet.LogStreamEvent.Log;

function isoTimestamp(ms: number) {
	return new Date(ms).toISOString().replace("T", " ").replace("Z", "");
}

interface LogTemplate {
	message: string;
	severity: "info" | "warn" | "error";
}

/** Log lines per actor name. Generic fallbacks keep unknown actors realistic. */
const LOG_TEMPLATES: Record<string, LogTemplate[]> = {
	ChatRoom: [
		{ message: "client connected conn=%h users=%n", severity: "info" },
		{
			message: "message id=%h bytes=%n broadcast to %n clients",
			severity: "info",
		},
		{ message: "persisted batch rows=%n took=%nms", severity: "info" },
		{
			message: "client disconnected conn=%h reason=idle",
			severity: "info",
		},
		{
			message: "slow consumer conn=%h backlog=%n dropping to delta sync",
			severity: "warn",
		},
	],
	UserPresence: [
		{ message: "heartbeat user=%h rooms=%n", severity: "info" },
		{ message: "typing start room=%h", severity: "info" },
		{ message: "fan-out presence to %n rooms took=%nms", severity: "info" },
	],
	Inbox: [
		{ message: "mention queued from=%h unread=%n", severity: "info" },
		{ message: "flushed %n notifications via push", severity: "info" },
	],
	RateLimiter: [
		{ message: "window rolled count=%n limit=120", severity: "info" },
		{ message: "throttled client=%h over by %n", severity: "warn" },
	],
	AgentSession: [
		{
			message: "turn=%n model=claude-sonnet tokens_in=%n",
			severity: "info",
		},
		{ message: "tool call shell exit=0 took=%nms", severity: "info" },
		{ message: "streamed %n tokens to %n clients", severity: "info" },
		{ message: "provider 429, retrying in %nms", severity: "warn" },
	],
};

const GENERIC_TEMPLATES: LogTemplate[] = [
	{ message: "action %a took=%nms", severity: "info" },
	{ message: "state saved bytes=%n took=%nms", severity: "info" },
	{ message: "connection opened conn=%h", severity: "info" },
	{ message: "connection closed conn=%h", severity: "info" },
	{ message: "retrying upstream after %nms", severity: "warn" },
];

const ACTIONS = ["send", "join", "leave", "sync", "update", "list"];

function fill(template: string, next: () => number) {
	return template
		.replace(/%h/g, () => hexId(next, 6))
		.replace(/%n/g, () => String(1 + Math.floor(next() * 480)))
		.replace(/%a/g, () => ACTIONS[Math.floor(next() * ACTIONS.length)]);
}

function entry(
	actor: MockActor,
	ts: number,
	message: string,
	severity: string,
	stream: "stdout" | "stderr",
	next: () => number,
): LogEntry {
	return {
		event: "log",
		data: {
			timestamp: isoTimestamp(ts),
			severity,
			message,
			region: actor.region,
			insertId: hexId(next, 16),
			stream,
		},
	};
}

/** Startup lines every actor emits, in order. */
function lifecycleLines(actor: MockActor, next: () => number): LogEntry[] {
	const start = actor.createdAt;
	return [
		entry(
			actor,
			start,
			`rivet: starting actor ${actor.name} key=${actor.key}`,
			"info",
			"stdout",
			next,
		),
		entry(
			actor,
			start + 12,
			"rivet: opened sqlite state (0 B)",
			"info",
			"stdout",
			next,
		),
		entry(
			actor,
			start + 41,
			`${actor.name}: ready`,
			"info",
			"stdout",
			next,
		),
	];
}

/**
 * Deterministic log history for an actor. Crashed actors end with a stack
 * trace; sleeping actors end with the sleep line.
 */
export function mockActorLogs(actor: MockActor, now = Date.now()): LogEntry[] {
	const next = rng(hash(actor.id));
	const templates = LOG_TEMPLATES[actor.name] ?? GENERIC_TEMPLATES;
	const lines = lifecycleLines(actor, next);
	const span = Math.max(now - actor.createdAt - 60, 1000);
	const count = 40 + Math.floor(next() * 60);
	for (let i = 0; i < count; i++) {
		const t = templates[Math.floor(next() * templates.length)];
		const ts =
			actor.createdAt + 60 + Math.floor((span * (i + 1)) / (count + 1));
		lines.push(
			entry(
				actor,
				ts,
				`${actor.name}: ${fill(t.message, next)}`,
				t.severity,
				t.severity === "info" ? "stdout" : "stderr",
				next,
			),
		);
	}
	if (actor.status === "crashed") {
		const ts = now - 2000;
		lines.push(
			entry(
				actor,
				ts,
				`${actor.name}: unhandled rejection in action handler`,
				"error",
				"stderr",
				next,
			),
			entry(
				actor,
				ts + 1,
				"TypeError: Cannot read properties of undefined (reading 'userId')",
				"error",
				"stderr",
				next,
			),
			entry(
				actor,
				ts + 1,
				`    at ${actor.name}.onMessage (file:///app/src/actors/${actor.name.toLowerCase()}.ts:84:31)`,
				"error",
				"stderr",
				next,
			),
			entry(
				actor,
				ts + 1,
				"    at Actor.dispatch (node_modules/rivetkit/dist/actor.js:412:18)",
				"error",
				"stderr",
				next,
			),
			entry(
				actor,
				ts + 40,
				"rivet: actor exited with code 1, restarting with backoff 4s",
				"error",
				"stderr",
				next,
			),
		);
	} else if (actor.status === "sleeping") {
		lines.push(
			entry(
				actor,
				now - 90_000,
				"rivet: no connections for 60s, persisting state (18 KB)",
				"info",
				"stdout",
				next,
			),
			entry(
				actor,
				now - 89_950,
				"rivet: actor sleeping",
				"info",
				"stdout",
				next,
			),
		);
	}
	return lines;
}

/** One new live line for a running actor, used to make the view tick. */
export function mockLiveLogLine(
	actor: MockActor,
	seq: number,
	now = Date.now(),
): LogEntry {
	const next = rng(hash(`${actor.id}:${seq}`));
	const templates = LOG_TEMPLATES[actor.name] ?? GENERIC_TEMPLATES;
	const t = templates[Math.floor(next() * templates.length)];
	return entry(
		actor,
		now,
		`${actor.name}: ${fill(t.message, next)}`,
		t.severity,
		t.severity === "info" ? "stdout" : "stderr",
		next,
	);
}
