export interface RivetActorIdentity {
	readonly actorId: string;
	readonly key: readonly string[];
	readonly name: string;
	readonly region: string;
}

export interface RivetContext {
	readonly identity: RivetActorIdentity;
	readonly env: Record<string, unknown>;
}

let currentContext: RivetContext | undefined;

export function runWithRivetContext<T>(context: RivetContext, callback: () => T): T {
	const previous = currentContext;
	currentContext = context;
	try {
		return callback();
	} finally {
		currentContext = previous;
	}
}

export function getRivetContext(): RivetContext {
	if (!currentContext) {
		throw new Error('[flue] Rivet context is only available while handling a Rivet actor request.');
	}
	return currentContext;
}

export function getRivetActorIdentity(): RivetActorIdentity {
	return getRivetContext().identity;
}
