import type { RankedMatchInfo } from "./menu.tsx";

interface RankedAssignmentEvent {
	matchId: string;
	username: string;
	rating: number;
	connId: string | null;
}

type AssignmentConnection = {
	getAssignment: (
		input: { username: string },
	) => Promise<RankedMatchInfo | null | Promise<RankedMatchInfo | null>>;
	on: (
		event: "assignmentReady",
		handler: (raw: unknown) => void,
	) => (() => void) | undefined;
};

function normalizeError(err: unknown): Error {
	if (err instanceof Error) return err;
	return new Error(String(err));
}

export async function waitForAssignment(
	mm: AssignmentConnection,
	username: string,
	expectedConnId?: string,
	timeoutMs = 120_000,
): Promise<RankedMatchInfo> {
	const existing = await readAssignment(mm, username);
	if (existing) return existing;

	return await new Promise<RankedMatchInfo>((resolve, reject) => {
		let settled = false;
		let timeout: number | null = null;
		let off: (() => void) | undefined;

		const settle = (fn: () => void) => {
			if (settled) return;
			settled = true;
			if (timeout !== null) window.clearTimeout(timeout);
			off?.();
			fn();
		};

		off = mm.on("assignmentReady", (raw: unknown) => {
			const next = raw as RankedAssignmentEvent;
			if (next.username !== username) return;
			// Bind assignment delivery to the queueing connection so duplicate usernames in
			// other tabs do not accept the wrong match assignment.
			if (expectedConnId && next.connId !== expectedConnId) return;
			settle(() => resolve(next));
		});

		timeout = window.setTimeout(() => {
			settle(() => reject(new Error("Timed out waiting for assignment")));
		}, timeoutMs);

		void readAssignment(mm, username)
			.then((next) => {
				if (!next) return;
				settle(() => resolve(next));
			})
			.catch((err) => {
				settle(() => reject(normalizeError(err)));
			});
	});
}

async function readAssignment(
	mm: AssignmentConnection,
	username: string,
): Promise<RankedMatchInfo | null> {
	const nested = await mm.getAssignment({ username });
	const resolved = await nested;
	if (!resolved) return null;
	return resolved as RankedMatchInfo;
}
