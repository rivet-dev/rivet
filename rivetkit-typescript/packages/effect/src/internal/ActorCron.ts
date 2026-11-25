import {
	Array as Arr,
	DateTime,
	Duration,
	Effect,
	Option,
	Record,
	Schema,
} from "effect";
import type * as Rivetkit from "rivetkit";
import type * as Action from "../Action.ts";
import * as RivetError from "../RivetError.ts";

type CronPayloadArgs<A extends Action.Any> = [Action.PayloadSchema<A>] extends [
	Schema.Void,
]
	? []
	: [payload: Action.PayloadConstructor<A>];

type CronServices<A extends Action.Any> =
	Action.PayloadSchema<A>["EncodingServices"];

type CronJobServices<Actions extends Action.Any> =
	Action.PayloadSchema<Actions>["DecodingServices"];

/** Options for `ActorCron.set`. */
export interface CronSetOptions {
	/** Identifies the job. Setting a job with a used name replaces it. */
	readonly name: string;
	/** A five-field cron expression, for example `"0 9 * * 1-5"`. */
	readonly expression: string;
	/** An IANA time zone, for example `"Europe/Berlin"`. Defaults to UTC. */
	readonly timezone?: string;
	/** Defaults to 100. Set to 0 to disable and clear history. Maximum 1,000. */
	readonly maxHistory?: number;
}

/** Options for `ActorCron.every`. */
export interface CronEveryOptions {
	/** Identifies the job. Setting a job with a used name replaces it. */
	readonly name: string;
	/** The fixed time between runs. Minimum 5 seconds. */
	readonly interval: Duration.Input;
	/** Defaults to 100. Set to 0 to disable and clear history. Maximum 1,000. */
	readonly maxHistory?: number;
}

/** A recurring job from `ActorCron`, with its payload decoded. */
export type CronJob<A extends Action.Any> =
	A extends Action.Action<
		infer Tag,
		infer Payload,
		infer _Success,
		infer _Error
	>
		? {
				readonly name: string;
				readonly _tag: Tag;
				readonly payload: Payload["Type"];
				readonly nextRunAt: DateTime.Utc;
				readonly lastRunAt: Option.Option<DateTime.Utc>;
				readonly maxHistory: number;
			} & (
				| {
						readonly kind: "cron";
						readonly expression: string;
						readonly timezone: string;
				  }
				| {
						readonly kind: "every";
						readonly interval: Duration.Duration;
				  }
			)
		: never;

/** One run of a recurring job. */
export interface CronFire {
	/** The tag of the action the job called. */
	readonly action: string;
	readonly scheduledAt: DateTime.Utc;
	readonly firedAt: DateTime.Utc;
	/** `None` while the run is in progress. */
	readonly finishedAt: Option.Option<DateTime.Utc>;
	readonly result: "running" | "ok" | "error" | "skipped";
	readonly error: Option.Option<{
		readonly group: string;
		readonly code: string;
		readonly message: string;
		readonly metadata?: unknown;
	}>;
}

/**
 * A stored recurring job that this actor cannot decode: its action is not one
 * of the actor's actions, or its payload fails the action's payload schema.
 */
export class CronJobDecodeError extends Schema.TaggedError<CronJobDecodeError>()(
	"CronJobDecodeError",
	{
		name: Schema.String,
		action: Schema.String,
		cause: Schema.optional(Schema.instanceOf(Schema.SchemaError)),
	},
) {
	override get message() {
		return `Cron job ${this.name} for ${this.action} does not decode`;
	}
}

/**
 * Runs this actor's own actions on a recurring schedule. RivetKit stores the
 * job, so it survives sleep, and runs it like a client call with
 * `connection: None`. Payloads are encoded with the action's JSON codec, the
 * same as `Actor.client`. An encoding failure is a defect. RivetKit
 * rejections, for example an invalid expression, fail with a `RivetError`.
 *
 * `get` and `list` decode payloads with the same codec. A stored job can stop
 * decoding, for example after a payload schema change or when raw RivetKit
 * code set it. `get` fails with a `CronJobDecodeError` for it, and `list`
 * skips it and logs a warning.
 */
export interface ActorCron<Actions extends Action.Any> {
	/** Calls `action` each time the cron expression matches. */
	readonly set: <A extends Actions>(
		options: CronSetOptions,
		action: A,
		...payload: CronPayloadArgs<A>
	) => Effect.Effect<void, RivetError.RivetError, CronServices<A>>;
	/** Calls `action` once per `interval`. */
	readonly every: <A extends Actions>(
		options: CronEveryOptions,
		action: A,
		...payload: CronPayloadArgs<A>
	) => Effect.Effect<void, RivetError.RivetError, CronServices<A>>;
	/** Succeeds with the job with this name, or `None`. */
	readonly get: (
		name: string,
	) => Effect.Effect<
		Option.Option<CronJob<Actions>>,
		RivetError.RivetError | CronJobDecodeError,
		CronJobServices<Actions>
	>;
	/** Succeeds with all jobs that decode. */
	readonly list: Effect.Effect<
		ReadonlyArray<CronJob<Actions>>,
		RivetError.RivetError,
		CronJobServices<Actions>
	>;
	/** Deletes a job and its history. Succeeds with `false` if no job has this name. */
	readonly delete: (
		name: string,
	) => Effect.Effect<boolean, RivetError.RivetError>;
	/** Succeeds with the job's most recent runs, newest first. */
	readonly history: (
		name: string,
		options?: { readonly limit?: number },
	) => Effect.Effect<ReadonlyArray<CronFire>, RivetError.RivetError>;
}

type RivetkitCron = Rivetkit.WakeContextOf<Rivetkit.AnyActorDefinition>["cron"];

type RivetkitCronJob = Awaited<ReturnType<RivetkitCron["list"]>>[number];

type RivetkitCronFire = Awaited<ReturnType<RivetkitCron["history"]>>[number];

const toCronFire = (fire: RivetkitCronFire): CronFire => ({
	action: fire.action,
	scheduledAt: DateTime.makeUnsafe(fire.scheduledAt),
	firedAt: DateTime.makeUnsafe(fire.firedAt),
	finishedAt: Option.map(
		Option.fromNullishOr(fire.finishedAt),
		DateTime.makeUnsafe,
	),
	result: fire.result,
	error: Option.fromNullishOr(fire.error),
});

/**
 * Builds the codecs for an actor's action payloads once, and returns a
 * function that binds them to one wake's RivetKit cron.
 */
export const makeActorCronFactory = <
	Actions extends Action.AnyWithProps,
>(actor: {
	readonly actions: ReadonlyArray<Actions>;
}) => {
	const encodePayloadByTag = Record.fromIterableWith(
		actor.actions,
		(action) => [
			action._tag,
			Schema.encodeEffect(Schema.toCodecJson(action.payloadSchema)),
		],
	);

	/**
	 * Reads the payload from the stored arguments the same way the action
	 * dispatcher does when the job fires.
	 */
	const decodeArgsByTag = Record.fromIterableWith(actor.actions, (action) => {
		const decodePayload = Schema.decodeUnknownEffect(
			Schema.toCodecJson(action.payloadSchema),
		);
		return [
			action._tag,
			(args: ReadonlyArray<unknown>) =>
				decodePayload(
					!action.hasPayload && args[0] === undefined
						? null
						: args[0],
				),
		];
	});

	/** A no-payload action gets no arguments, like a raw RivetKit call. */
	const encodeCronArgs = (
		action: Action.AnyWithProps,
		payload: ReadonlyArray<unknown>,
	): Effect.Effect<Array<unknown>, never, any> => {
		if (!action.hasPayload) return Effect.succeed([]);

		return encodePayloadByTag[action._tag](payload[0]).pipe(
			Effect.map((encodedPayload) => [encodedPayload]),
			Effect.orDie,
		);
	};

	const decodeCronJob = (job: RivetkitCronJob) =>
		Option.match(Record.get(decodeArgsByTag, job.action), {
			onNone: () =>
				Effect.fail(
					new CronJobDecodeError({
						name: job.name,
						action: job.action,
					}),
				),
			onSome: (decodeArgs) =>
				decodeArgs(job.args).pipe(
					Effect.map((payload) => ({
						name: job.name,
						_tag: job.action,
						payload,
						nextRunAt: DateTime.makeUnsafe(job.nextRunAt),
						lastRunAt: Option.map(
							Option.fromNullishOr(job.lastRunAt),
							DateTime.makeUnsafe,
						),
						maxHistory: job.maxHistory,
						...(job.kind === "cron"
							? {
									kind: job.kind,
									expression: job.expression,
									timezone: job.timezone,
								}
							: {
									kind: job.kind,
									interval: Duration.millis(job.interval),
								}),
					})),
					Effect.mapError(
						(cause) =>
							new CronJobDecodeError({
								name: job.name,
								action: job.action,
								cause,
							}),
					),
				),
		});

	return (rivetkitCron: RivetkitCron): ActorCron<Actions> => {
		const set = (
			options: CronSetOptions,
			action: Action.AnyWithProps,
			...payload: ReadonlyArray<unknown>
		) =>
			encodeCronArgs(action, payload).pipe(
				Effect.flatMap((args) =>
					Effect.tryPromise({
						try: () =>
							rivetkitCron.set({
								...options,
								action: action._tag,
								args,
							}),
						catch: RivetError.fromUnknown,
					}),
				),
			);

		const every = (
			options: CronEveryOptions,
			action: Action.AnyWithProps,
			...payload: ReadonlyArray<unknown>
		) =>
			encodeCronArgs(action, payload).pipe(
				Effect.flatMap((args) =>
					Effect.tryPromise({
						try: () =>
							rivetkitCron.every({
								...options,
								interval: Duration.toMillis(options.interval),
								action: action._tag,
								args,
							}),
						catch: RivetError.fromUnknown,
					}),
				),
			);

		const get = (name: string) =>
			Effect.tryPromise({
				try: () => rivetkitCron.get(name),
				catch: RivetError.fromUnknown,
			}).pipe(
				Effect.flatMap((job) =>
					job === undefined
						? Effect.succeedNone
						: Effect.asSome(decodeCronJob(job)),
				),
			);

		const list = Effect.tryPromise({
			try: () => rivetkitCron.list(),
			catch: RivetError.fromUnknown,
		}).pipe(
			Effect.flatMap((jobs) =>
				Effect.forEach(jobs, (job) =>
					decodeCronJob(job).pipe(
						Effect.asSome,
						Effect.catchTag("CronJobDecodeError", (error) =>
							Effect.logWarning(
								"Skipped a cron job that does not decode",
								error,
							).pipe(
								Effect.annotateLogs({
									cronName: error.name,
									action: error.action,
								}),
								Effect.as(Option.none()),
							),
						),
					),
				),
			),
			Effect.map(Arr.getSomes),
		);

		const deleteJob = (name: string) =>
			Effect.tryPromise({
				try: () => rivetkitCron.delete(name),
				catch: RivetError.fromUnknown,
			});

		const history = (name: string, options?: { readonly limit?: number }) =>
			Effect.tryPromise({
				try: () => rivetkitCron.history(name, options),
				catch: RivetError.fromUnknown,
			}).pipe(Effect.map(Arr.map(toCronFire)));

		/**
		 * The public signatures tie each payload to its action. The
		 * implementation looks the codecs up by tag instead.
		 */
		return {
			set,
			every,
			get,
			list,
			delete: deleteJob,
			history,
		} as unknown as ActorCron<Actions>;
	};
};
