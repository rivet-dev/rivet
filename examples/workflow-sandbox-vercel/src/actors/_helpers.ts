// Type helper - cast loop context to access actor-specific properties
// Only call these helpers INSIDE a step callback where state access is allowed
// biome-ignore lint/suspicious/noExplicitAny: Workflow context typing workaround
export type ActorLoopContext<S> = {
	state: S;
	broadcast: (name: string, ...args: unknown[]) => void;
};

// biome-ignore lint/suspicious/noExplicitAny: Workflow context typing workaround
export function actorCtx<S>(ctx: unknown): ActorLoopContext<S> {
	return ctx as any;
}
