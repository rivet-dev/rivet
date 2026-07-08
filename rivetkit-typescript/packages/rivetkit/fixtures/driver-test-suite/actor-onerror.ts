// @ts-nocheck
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { actor, queue } from "rivetkit";
import type { ActorErrorContext, ActorErrorEvent } from "../../src/actor/config";

export interface ActorOnErrorRecord {
	source: "actor" | "registry" | "throwing-actor" | "rejecting-actor";
	actorId: string;
	name: string;
	key?: string;
	arm: "action" | "hook" | "queue" | "internal" | "fatal";
	detail?: string;
	scheduled?: boolean;
	errorMessage: string;
	errorName?: string;
	errorGroup?: string;
	errorCode?: string;
	rawError: boolean;
}

const reports = new Map<string, ActorOnErrorRecord[]>();
const reportDir = join("/tmp", "rivetkit-actor-onerror-reports");

function reportFile(key: string): string {
	return join(reportDir, `${encodeURIComponent(key)}.json`);
}

function reportKey(c: ActorErrorContext): string {
	return c.key ?? c.actorId;
}

function readEvent(event: ActorErrorEvent): {
	arm: ActorOnErrorRecord["arm"];
	data: Record<string, unknown> & { error: unknown };
} {
	const [arm, data] = Object.entries(event)[0] as [
		ActorOnErrorRecord["arm"],
		Record<string, unknown> & { error: unknown },
	];
	return { arm, data };
}

export function recordActorOnError(
	source: ActorOnErrorRecord["source"],
	c: ActorErrorContext,
	event: ActorErrorEvent,
): void {
	const { arm, data } = readEvent(event);
	const error = data.error as {
		message?: string;
		name?: string;
		group?: string;
		code?: string;
	};
	const record: ActorOnErrorRecord = {
		source,
		actorId: c.actorId,
		name: c.name,
		key: c.key,
		arm,
		detail:
			typeof data.name === "string"
				? data.name
				: typeof data.kind === "string"
					? data.kind
					: typeof data.phase === "string"
						? data.phase
						: undefined,
		scheduled:
			typeof data.scheduled === "boolean" ? data.scheduled : undefined,
		errorMessage:
			typeof error?.message === "string"
				? error.message
				: String(data.error),
		errorName: typeof error?.name === "string" ? error.name : undefined,
		errorGroup:
			typeof error?.group === "string" ? error.group : undefined,
		errorCode: typeof error?.code === "string" ? error.code : undefined,
		rawError: data.error instanceof Error,
	};
	const key = reportKey(c);
	reports.set(key, [...(reports.get(key) ?? []), record]);
	const next = [...readReportFile(key), record];
	mkdirSync(reportDir, { recursive: true });
	writeFileSync(reportFile(key), JSON.stringify(next), "utf8");
}

export function resetActorOnErrorReports(key?: string): void {
	if (key === undefined) {
		reports.clear();
		rmSync(reportDir, { force: true, recursive: true });
	} else {
		reports.delete(key);
		rmSync(reportFile(key), { force: true });
	}
}

export function getActorOnErrorReports(key: string): ActorOnErrorRecord[] {
	const fileReports = readReportFile(key);
	return fileReports.length > 0 ? fileReports : [...(reports.get(key) ?? [])];
}

function readReportFile(key: string): ActorOnErrorRecord[] {
	try {
		return JSON.parse(readFileSync(reportFile(key), "utf8"));
	} catch {
		return [];
	}
}

export const actorOnErrorActionActor = actor({
	state: {
		changed: 0,
	},
	onError: (c, event) => {
		recordActorOnError("actor", c, event);
	},
	actions: {
		failAction: () => {
			throw new Error("onError action boom");
		},
		succeed: () => "ok",
		scheduleFailure: (c, delayMs: number) => {
			c.schedule.after(delayMs, "scheduledFailure");
			return true;
		},
		scheduledFailure: () => {
			throw new Error("onError scheduled boom");
		},
		breakPersist: (c) => {
			c.state.changed++;
			c.state.unserializable = () => "nope";
			return true;
		},
	},
	options: {
		actionTimeout: 200,
	},
});

export const actorOnErrorTimeoutActor = actor({
	onError: (c, event) => {
		recordActorOnError("actor", c, event);
	},
	actions: {
		timeoutAction: async () => {
			await new Promise((resolve) => setTimeout(resolve, 5_000));
		},
	},
	options: {
		actionTimeout: 100,
	},
});

export const actorOnErrorHookActor = actor({
	onError: (c, event) => {
		recordActorOnError("actor", c, event);
	},
	onRequest: (_c, _request) => {
		throw new Error("onError request boom");
	},
	actions: {},
});

export const actorOnErrorStartupActor = actor({
	onError: (c, event) => {
		recordActorOnError("actor", c, event);
	},
	onCreate: () => {
		throw new Error("onError startup boom");
	},
	actions: {
		ping: () => "pong",
	},
});

export const actorOnErrorQueueActor = actor({
	queues: {
		fail: queue<{ value: number }>({
			canPublish: () => {
				throw new Error("onError queue boom");
			},
		}),
	},
	onError: (c, event) => {
		recordActorOnError("actor", c, event);
	},
	actions: {},
});

export const actorOnErrorRunActor = actor({
	state: {
		runStarted: false,
	},
	onError: (c, event) => {
		recordActorOnError("actor", c, event);
	},
	run: async (c) => {
		c.state.runStarted = true;
		await new Promise((resolve) => setTimeout(resolve, 50));
		throw new Error("onError run boom");
	},
	actions: {
		ping: (c) => c.state.runStarted,
	},
});

export const actorOnErrorThrowingHookActor = actor({
	onError: (c, event) => {
		recordActorOnError("throwing-actor", c, event);
		throw new Error("onError hook threw");
	},
	actions: {
		failAction: () => {
			throw new Error("onError original after throw");
		},
		ping: () => "pong",
	},
});

export const actorOnErrorRejectingHookActor = actor({
	onError: async (c, event) => {
		recordActorOnError("rejecting-actor", c, event);
		await Promise.resolve();
		throw new Error("onError hook rejected");
	},
	actions: {
		failAction: () => {
			throw new Error("onError original after reject");
		},
		ping: () => "pong",
	},
});

export const actorOnErrorAbortRunActor = actor({
	onError: (c, event) => {
		recordActorOnError("actor", c, event);
	},
	run: async (c) => {
		await new Promise((_resolve, reject) => {
			c.abortSignal.addEventListener(
				"abort",
				() => reject(Object.assign(new Error("aborted"), {
					group: "actor",
					code: "aborted",
				})),
				{ once: true },
			);
		});
	},
	actions: {
		destroySelf: (c) => {
			c.destroy();
		},
		ping: () => "pong",
	},
	options: {
		sleepTimeout: 50,
	},
});
