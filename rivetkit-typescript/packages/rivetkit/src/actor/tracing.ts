/** Sample rates keyed by action name, nested like the actor's action groups. */
export interface ActionSampleRates {
	readonly [name: string]: number | ActionSampleRates;
}

/**
 * {@link ActionSampleRates} limited to the actions of one actor, so a rate for
 * an action that does not exist is a compile error.
 */
export type ActionTraceSamplers<TActions> = {
	readonly [K in keyof TActions]?: TActions[K] extends (
		...args: never[]
	) => unknown
		? number
		: ActionTraceSamplers<TActions[K]>;
};

/** Trace sampling for one actor definition. */
export interface ActorTracingOptions<TActions> {
	/**
	 * Share of this actor's invocations that are recorded, from 0 to 1. A rate
	 * lowers what is recorded and never raises it, so an invocation inside a
	 * trace the caller did not record is not recorded at any rate.
	 */
	readonly sampler?: number;
	/** Rates that replace `sampler` for single actions. */
	readonly actions?: ActionTraceSamplers<TActions>;
}
