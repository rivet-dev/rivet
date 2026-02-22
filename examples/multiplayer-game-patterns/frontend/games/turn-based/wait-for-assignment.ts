interface Assignment {
	playerId: string;
}

type AssignmentConnection = {
	getAssignment: (
		input: { playerId: string },
	) => Promise<Assignment | null | Promise<Assignment | null>>;
	on: (
		event: "assignmentReady",
		handler: (raw: unknown) => void,
	) => (() => void) | undefined;
};

function normalizeError(err: unknown): Error {
	if (err instanceof Error) return err;
	return new Error(String(err));
}

export async function waitForAssignment<T extends Assignment>(
	mm: AssignmentConnection,
	playerId: string,
	timeoutMs = 120_000,
): Promise<T> {
	const existing = await readAssignment<T>(mm, playerId);
	if (existing) return existing as T;

	return await new Promise<T>((resolve, reject) => {
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
			const next = raw as T;
			if (next.playerId !== playerId) return;
			settle(() => resolve(next));
		});

		timeout = window.setTimeout(() => {
			settle(() => reject(new Error("Timed out waiting for assignment")));
		}, timeoutMs);

		void readAssignment<T>(mm, playerId)
			.then((next) => {
				if (!next) return;
				settle(() => resolve(next));
			})
			.catch((err) => {
				settle(() => reject(normalizeError(err)));
			});
	});
}

async function readAssignment<T extends Assignment>(
	mm: AssignmentConnection,
	playerId: string,
): Promise<T | null> {
	const nested = await mm.getAssignment({ playerId });
	const resolved = await nested;
	if (!resolved) return null;
	return resolved as T;
}
