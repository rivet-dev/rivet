import * as Context from "effect/Context";
import type * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import { type Pipeable, pipeArguments } from "effect/Pipeable";
import type * as PubSub from "effect/PubSub";
import type * as Queue from "effect/Queue";
import * as Schema from "effect/Schema";
import type * as Scope from "effect/Scope";
import type * as Stream from "effect/Stream";
import type * as SubscriptionRef from "effect/SubscriptionRef";
import type * as Action from "./Action";
import type * as Message from "./Message";

const TypeId = "~@rivetkit/effect/Actor";

/**
 * Schemas keyed by the event names an actor can publish.
 */
export type EventSchemas = Record<string, Schema.Top>;

/**
 * Display and runtime options carried by an actor contract.
 */
export interface Options {
	readonly name?: string;
	readonly icon?: string;
	readonly maxQueueSize?: number;
	readonly maxQueueMessageSize?: number;
}

/**
 * Initial implementation uses Effect's SubscriptionRef directly. The
 * persisted variant is defined by a separate module.
 */
export type StateRef<A> = SubscriptionRef.SubscriptionRef<A>;

/**
 * Effect-shaped KV service available inside an actor wake scope.
 */
export interface KvStore {
	readonly get: <A = unknown>(
		key: string | Uint8Array,
	) => Effect.Effect<A | null>;
	readonly put: (
		key: string | Uint8Array,
		value: string | Uint8Array | ArrayBuffer,
	) => Effect.Effect<void>;
	readonly delete: (key: string | Uint8Array) => Effect.Effect<void>;
	readonly batchGet: (
		keys: ReadonlyArray<Uint8Array>,
	) => Effect.Effect<ReadonlyArray<Uint8Array | null>>;
	readonly batchPut: (
		entries: ReadonlyArray<readonly [Uint8Array, Uint8Array]>,
	) => Effect.Effect<void>;
	readonly batchDelete: (
		keys: ReadonlyArray<Uint8Array>,
	) => Effect.Effect<void>;
	readonly deleteRange: (
		start: Uint8Array,
		end: Uint8Array,
	) => Effect.Effect<void>;
}

/**
 * Minimal Effect-shaped database service available inside an actor wake scope.
 */
export interface DbClient {
	readonly execute: <A = Record<string, unknown>>(
		query: string,
		...args: ReadonlyArray<unknown>
	) => Effect.Effect<ReadonlyArray<A>>;
}

export interface KvService {
	readonly _: unique symbol;
}

export interface DbService {
	readonly _: unique symbol;
}

export interface RegistryShape {
	readonly _: unique symbol;
}

export interface ActorTransportOptions {
	readonly endpoint: string;
	readonly token?: string;
}

export interface ActorTransportShape extends ActorTransportOptions {
	readonly _: unique symbol;
}

export interface StateService<out Name extends string> {
	readonly _: unique symbol;
	readonly name: Name;
}

export interface EventsService<out Name extends string> {
	readonly _: unique symbol;
	readonly name: Name;
}

export interface MessagesService<out Name extends string> {
	readonly _: unique symbol;
	readonly name: Name;
}

export const Kv: Context.Service<KvService, KvStore> = Context.Service(
	"@rivetkit/effect/Actor/Kv",
);

export const Db: Context.Service<DbService, DbClient> = Context.Service(
	"@rivetkit/effect/Actor/Db",
);

export class Registry extends Context.Service<Registry, RegistryShape>()(
	"@rivetkit/effect/Actor/Registry",
) {}

export class ActorTransport extends Context.Service<
	ActorTransport,
	ActorTransportShape
>()("@rivetkit/effect/Actor/ActorTransport") {
	static layer(options: ActorTransportOptions): Layer.Layer<ActorTransport> {
		return Layer.succeed(ActorTransport, {
			...options,
			_: undefined as never,
		});
	}
}

export type EventPubSubMap<Events extends EventSchemas> = {
	readonly [Name in keyof Events & string]: PubSub.PubSub<Events[Name]["Type"]>;
};

type EventDecodeServices<Events extends EventSchemas> = {
	readonly [Name in keyof Events]: Events[Name]["DecodingServices"];
}[keyof Events];

type EventEncodeServices<Events extends EventSchemas> = {
	readonly [Name in keyof Events]: Events[Name]["EncodingServices"];
}[keyof Events];

type CompleteArgs<A> = undefined extends A
	? readonly [value?: A]
	: readonly [value: A];

export type MessageQueueItem<M extends Message.AnyWithProps> =
	M extends Message.Message<infer Tag, infer Payload, infer Success>
		? {
				readonly _tag: Tag;
				readonly message: M;
				readonly payload: Payload["Type"];
			} & ([Success] extends [typeof Schema.Never]
				? object
				: {
						readonly complete: (
							...args: CompleteArgs<Success["Type"]>
						) => Effect.Effect<void>;
					})
		: never;

export type ActionRequest<A extends Action.AnyWithProps> =
	A extends Action.Action<infer Tag, infer Payload, infer _Success, infer _Error>
		? {
				readonly _tag: Tag;
				readonly action: A;
				readonly payload: Payload["Type"];
			}
		: never;

export type ActionHandlers<Actions extends Action.AnyWithProps> = {
	readonly [Tag in Action.Tag<Actions>]: (
		request: ActionRequest<Action.ExtractTag<Actions, Tag>>,
	) => Effect.Effect<
		Action.Success<Action.ExtractTag<Actions, Tag>>,
		Action.Error<Action.ExtractTag<Actions, Tag>>,
		any
	>;
};

type HandlerServices<Handlers> = {
	readonly [Name in keyof Handlers]: Handlers[Name] extends (
		...args: ReadonlyArray<any>
	) => Effect.Effect<any, any, infer R>
		? R
		: never;
}[keyof Handlers];

export interface AbortSignalLike {
	readonly aborted: boolean;
	readonly reason?: unknown;
}

export interface CallOptions {
	readonly signal?: AbortSignalLike;
}

export type ActorKey = string | ReadonlyArray<string>;

export interface GetOptions {
	readonly params?: unknown;
	readonly getParams?: () => Effect.Effect<unknown>;
	readonly signal?: AbortSignalLike;
}

export interface GetOrCreateOptions extends GetOptions {
	readonly createInRegion?: string;
	readonly createWithInput?: unknown;
}

export interface CreateOptions extends GetOptions {
	readonly region?: string;
	readonly input?: unknown;
}

type ActionClientArgs<A extends Action.AnyWithProps> = [
	Action.PayloadConstructor<A>,
] extends [void]
	? readonly [payload?: Action.PayloadConstructor<A>, options?: CallOptions]
	: readonly [payload: Action.PayloadConstructor<A>, options?: CallOptions];

type ActionClientMethod<A extends Action.AnyWithProps> = (
	...args: ActionClientArgs<A>
) => Effect.Effect<Action.Success<A>, Action.Error<A>>;

type EnvelopeSuccess<E extends Message.AnyEnvelope> = E extends Message.Envelope<
	string,
	Schema.Top,
	infer Success
>
	? [Success] extends [typeof Schema.Never]
		? void
		: Success["Type"]
	: never;

export type ActorHandle<
	Actions extends Action.AnyWithProps,
	Messages extends Message.AnyWithProps,
	Events extends EventSchemas,
> = {
	readonly [Tag in Action.Tag<Actions>]: ActionClientMethod<
		Action.ExtractTag<Actions, Tag>
	>;
} & {
	readonly send: <Envelope extends Message.EnvelopeOf<Messages>>(
		envelope: Envelope,
		options?: CallOptions,
	) => Effect.Effect<EnvelopeSuccess<Envelope>>;
	readonly subscribe: <Name extends keyof Events & string>(
		name: Name,
	) => Stream.Stream<Events[Name]["Type"]>;
};

export interface ActorClient<
	Actions extends Action.AnyWithProps,
	Messages extends Message.AnyWithProps,
	Events extends EventSchemas,
> {
	readonly get: (
		key?: ActorKey,
		options?: GetOptions,
	) => ActorHandle<Actions, Messages, Events>;
	readonly getOrCreate: (
		key?: ActorKey,
		options?: GetOrCreateOptions,
	) => ActorHandle<Actions, Messages, Events>;
	readonly getForId: (
		actorId: string,
		options?: GetOptions,
	) => ActorHandle<Actions, Messages, Events>;
	readonly create: (
		key?: ActorKey,
		options?: CreateOptions,
	) => Effect.Effect<ActorHandle<Actions, Messages, Events>>;
}

/**
 * A Rivet Actor contract. It carries schemas and generated Effect service
 * tags, but no server implementation.
 */
export interface Actor<
	Name extends string,
	State extends Schema.Top = Schema.Void,
	Actions extends Action.AnyWithProps = never,
	Messages extends Message.AnyWithProps = never,
	Events extends EventSchemas = {},
> extends Pipeable {
	readonly [TypeId]: typeof TypeId;
	readonly _tag: Name;
	readonly key: string;
	readonly stateSchema: State;
	readonly actions: ReadonlyArray<Actions>;
	readonly messages: ReadonlyArray<Messages>;
	readonly events: Events;
	readonly options: Options;
	readonly annotations: Context.Context<never>;
	readonly State: Context.Service<StateService<Name>, StateRef<State["Type"]>>;
	readonly Events: Context.Service<EventsService<Name>, EventPubSubMap<Events>>;
	readonly Messages: Context.Service<
		MessagesService<Name>,
		Queue.Dequeue<MessageQueueItem<Messages>>
	>;
	readonly client: Effect.Effect<
		ActorClient<Actions, Messages, Events>,
		never,
		ActorTransport | ClientServices<Actor<Name, State, Actions, Messages, Events>>
	>;

	of<Handlers extends ActionHandlers<Actions>>(handlers: Handlers): Handlers;

	toLayer<Handlers extends ActionHandlers<Actions>, RX = never>(
		build: Handlers | Effect.Effect<Handlers, never, RX>,
	): Layer.Layer<
		never,
		never,
		| Exclude<
				RX,
				| StateService<Name>
				| EventsService<Name>
				| MessagesService<Name>
				| KvService
				| DbService
				| Scope.Scope
			>
		| HandlerServices<Handlers>
		| Action.ServicesServer<Actions>
		| Action.ServicesClient<Actions>
		| Message.ServicesServer<Messages>
		| Message.ServicesClient<Messages>
		| Registry
	>;

	annotate<I, S>(
		tag: Context.Key<I, S>,
		value: S,
	): Actor<Name, State, Actions, Messages, Events>;

	annotateMerge<I>(
		annotations: Context.Context<I>,
	): Actor<Name, State, Actions, Messages, Events>;
}

/**
 * Type-erased view of any actor contract.
 */
export interface Any extends Pipeable {
	readonly [TypeId]: typeof TypeId;
	readonly _tag: string;
	readonly key: string;
}

/**
 * Type-erased actor with all runtime properties available.
 */
export interface AnyWithProps
	extends Actor<
		string,
		Schema.Top,
		Action.AnyWithProps,
		Message.AnyWithProps,
		EventSchemas
	> {}

export type Name<A> = A extends Actor<infer _Name, any, any, any, any>
	? _Name
	: never;

export type StateSchema<A> = A extends Actor<any, infer _State, any, any, any>
	? _State
	: never;

export type State<A> = StateSchema<A>["Type"];

export type Actions<A> = A extends Actor<any, any, infer _Actions, any, any>
	? _Actions
	: never;

export type Messages<A> = A extends Actor<any, any, any, infer _Messages, any>
	? _Messages
	: never;

export type Events<A> = A extends Actor<any, any, any, any, infer _Events>
	? _Events
	: never;

export type EventName<A> = keyof Events<A> & string;

export type EventPayload<
	A,
	Name extends EventName<A>,
> = Events<A>[Name]["Type"];

export type ProvidedServices<A> = A extends Actor<
	infer _Name,
	any,
	any,
	any,
	any
>
	?
			| StateService<_Name>
			| EventsService<_Name>
			| MessagesService<_Name>
			| KvService
			| DbService
	: never;

export type Services<A> = A extends Actor<
	any,
	infer _State,
	infer _Actions,
	infer _Messages,
	infer _Events
>
	?
			| _State["DecodingServices"]
			| _State["EncodingServices"]
			| Action.Services<_Actions>
			| Message.Services<_Messages>
			| EventDecodeServices<_Events>
			| EventEncodeServices<_Events>
	: never;

export type ClientServices<A> = A extends Actor<
	any,
	any,
	infer _Actions,
	infer _Messages,
	infer _Events
>
	?
			| Action.ServicesClient<_Actions>
			| Message.ServicesClient<_Messages>
			| EventDecodeServices<_Events>
	: never;

export type ServerServices<A> = A extends Actor<
	any,
	infer _State,
	infer _Actions,
	infer _Messages,
	infer _Events
>
	?
			| _State["DecodingServices"]
			| _State["EncodingServices"]
			| Action.ServicesServer<_Actions>
			| Message.ServicesServer<_Messages>
			| EventEncodeServices<_Events>
	: never;

export const isActor = (u: unknown): u is Any =>
	typeof u === "object" && u !== null && TypeId in u;

const identity = <A>(value: A): A => value;

const Proto = {
	[TypeId]: TypeId,
	pipe() {
		// biome-ignore lint/complexity/noArguments: required by Effect's Pipeable contract
		return pipeArguments(this, arguments);
	},
	of: identity,
	toLayer(this: AnyWithProps) {
		throw new Error(
			`Actor.toLayer for ${this._tag} is not yet implemented. Registry runtime wiring is pending.`,
		);
	},
	get client(): never {
		const self = this as unknown as AnyWithProps;
		throw new Error(
			`Actor.client for ${self._tag} is not yet implemented. ActorTransport runtime wiring is pending.`,
		);
	},
	annotate(this: AnyWithProps, tag: Context.Key<any, any>, value: any) {
		return makeProto({
			_tag: this._tag,
			stateSchema: this.stateSchema,
			actions: this.actions,
			messages: this.messages,
			events: this.events,
			options: this.options,
			annotations: Context.add(this.annotations, tag, value),
		});
	},
	annotateMerge(this: AnyWithProps, annotations: Context.Context<any>) {
		return makeProto({
			_tag: this._tag,
			stateSchema: this.stateSchema,
			actions: this.actions,
			messages: this.messages,
			events: this.events,
			options: this.options,
			annotations: Context.merge(this.annotations, annotations),
		});
	},
};

const makeProto = <
	const Name extends string,
	State extends Schema.Top,
	Actions extends Action.AnyWithProps,
	Messages extends Message.AnyWithProps,
	Events extends EventSchemas,
>(options: {
	readonly _tag: Name;
	readonly stateSchema: State;
	readonly actions: ReadonlyArray<Actions>;
	readonly messages: ReadonlyArray<Messages>;
	readonly events: Events;
	readonly options: Options;
	readonly annotations: Context.Context<never>;
}): Actor<Name, State, Actions, Messages, Events> => {
	const key = `rivetkit/effect/Actor/${options._tag}`;
	const StateTag = Context.Service<StateService<Name>, StateRef<State["Type"]>>(
		`${key}/State`,
	);
	const EventsTag = Context.Service<
		EventsService<Name>,
		EventPubSubMap<Events>
	>(`${key}/Events`);
	const MessagesTag = Context.Service<
		MessagesService<Name>,
		Queue.Dequeue<MessageQueueItem<Messages>>
	>(`${key}/Messages`);
	return Object.assign(Object.create(Proto), {
		...options,
		key,
		State: StateTag,
		Events: EventsTag,
		Messages: MessagesTag,
	}) as Actor<Name, State, Actions, Messages, Events>;
};

/**
 * Define a Rivet Actor contract.
 */
export const make = <
	const Name extends string,
	State extends Schema.Top | Schema.Struct.Fields = Schema.Void,
	const Actions extends ReadonlyArray<Action.AnyWithProps> = readonly [],
	const Messages extends ReadonlyArray<Message.AnyWithProps> = readonly [],
	const Events extends EventSchemas = {},
>(
	name: Name,
	options?: {
		readonly state?: State;
		readonly actions?: Actions;
		readonly messages?: Messages;
		readonly events?: Events;
		readonly options?: Options;
	},
): Actor<
	Name,
	State extends Schema.Struct.Fields ? Schema.Struct<State> : State,
	Actions[number],
	Messages[number],
	Events
> => {
	const stateSchema: Schema.Top = Schema.isSchema(options?.state)
		? (options?.state as any)
		: options?.state
			? Schema.Struct(options?.state as any)
			: Schema.Void;
	return makeProto({
		_tag: name,
		stateSchema,
		actions: (options?.actions ?? []) as ReadonlyArray<Action.AnyWithProps>,
		messages: (options?.messages ?? []) as ReadonlyArray<Message.AnyWithProps>,
		events: (options?.events ?? {}) as EventSchemas,
		options: options?.options ?? {},
		annotations: Context.empty(),
	}) as any;
};
