import * as Context from "effect/Context";
import type * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import * as Predicate from "effect/Predicate";
import type * as Scope from "effect/Scope";
import type * as Action from "./Action";

const TypeId = "~@rivetkit/effect/Actor";

export const isActor = (u: unknown): u is Actor<any, any> =>
	Predicate.hasProperty(u, TypeId);

/**
 * Display options carried by an actor contract.
 */
export interface Options {
	readonly name?: string;
	readonly icon?: string;
}

export interface RegistryShape {
	readonly _: unique symbol;
}

export interface RegistryOptions {
	readonly storagePath: string;
}

export interface ClientOptions {
	readonly endpoint: string;
	readonly token?: string;
}

export interface ClientShape extends ClientOptions {
	readonly _: unique symbol;
}

export class Registry extends Context.Service<Registry, RegistryShape>()(
	"@rivetkit/effect/Actor/Registry",
) {
	static layer(_options: RegistryOptions): Layer.Layer<Registry> {
		return Layer.sync(Registry, () => {
			throw new Error(
				"Registry.layer is not yet implemented. Engine wiring is pending.",
			);
		});
	}
}

export class Client extends Context.Service<Client, ClientShape>()(
	"@rivetkit/effect/Client",
) {
	static layer(options: ClientOptions): Layer.Layer<Client> {
		return Layer.succeed(Client, {
			...options,
			_: undefined as never,
		});
	}
}

export type ActionRequest<A extends Action.AnyWithProps> =
	A extends Action.Action<infer Tag, infer Payload, infer _Success, infer _Error>
		? {
				readonly _tag: Tag;
				readonly action: A;
				readonly payload: Payload["Type"];
			}
		: never;

export type ActionHandlers<Actions extends Action.AnyWithProps> = {
	readonly [A in Actions as Action.Tag<A>]: (
		request: ActionRequest<A>,
	) => Effect.Effect<Action.Success<A>, Action.Error<A>, any>;
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

export type ActorHandle<Actions extends Action.AnyWithProps> = {
	readonly [A in Actions as Action.Tag<A>]: ActionClientMethod<A>;
};

export interface ActorClient<Actions extends Action.AnyWithProps> {
	readonly get: (
		key?: ActorKey,
		options?: GetOptions,
	) => ActorHandle<Actions>;
	readonly getOrCreate: (
		key?: ActorKey,
		options?: GetOrCreateOptions,
	) => ActorHandle<Actions>;
	readonly getForId: (
		actorId: string,
		options?: GetOptions,
	) => ActorHandle<Actions>;
	readonly create: (
		key?: ActorKey,
		options?: CreateOptions,
	) => Effect.Effect<ActorHandle<Actions>>;
}

/**
 * A Rivet Actor contract. It carries the action schemas and
 * display options, but no server implementation.
 */
export interface Actor<
	Name extends string,
	Actions extends Action.AnyWithProps = never,
> {
	readonly [TypeId]: typeof TypeId;
	readonly _tag: Name;
	readonly key: string;
	readonly actions: ReadonlyArray<Actions>;
	readonly options: Options;
	readonly client: Effect.Effect<
		ActorClient<Actions>,
		never,
		Client | ClientServices<Actor<Name, Actions>>
	>;

	of<Handlers extends ActionHandlers<Actions>>(handlers: Handlers): Handlers;

	toLayer<Handlers extends ActionHandlers<Actions>, RX = never>(
		build: Handlers | Effect.Effect<Handlers, never, RX>,
	): Layer.Layer<
		never,
		never,
		| Exclude<RX, Scope.Scope>
		| HandlerServices<Handlers>
		| Action.ServicesServer<Actions>
		| Action.ServicesClient<Actions>
		| Registry
	>;
}

/**
 * Type-erased view of any actor contract.
 */
export interface Any {
	readonly [TypeId]: typeof TypeId;
	readonly _tag: string;
	readonly key: string;
}

/**
 * Type-erased actor with all runtime properties available.
 */
export interface AnyWithProps extends Actor<string, Action.AnyWithProps> {}

export type Name<A> = A extends Actor<infer _Name, any> ? _Name : never;

export type Actions<A> = A extends Actor<any, infer _Actions> ? _Actions : never;

export type Services<A> = A extends Actor<any, infer _Actions>
	? Action.Services<_Actions>
	: never;

export type ClientServices<A> = A extends Actor<any, infer _Actions>
	? Action.ServicesClient<_Actions>
	: never;

export type ServerServices<A> = A extends Actor<any, infer _Actions>
	? Action.ServicesServer<_Actions>
	: never;

const identity = <A>(value: A): A => value;

const Proto = {
	[TypeId]: TypeId,
	of: identity,
	toLayer(this: AnyWithProps) {
		throw new Error(
			`Actor.toLayer for ${this._tag} is not yet implemented. Registry runtime wiring is pending.`,
		);
	},
	get client(): never {
		const self = this as unknown as AnyWithProps;
		throw new Error(
			`Actor.client for ${self._tag} is not yet implemented. Client runtime wiring is pending.`,
		);
	},
};

const makeProto = <
	const Name extends string,
	Actions extends Action.AnyWithProps,
>(options: {
	readonly _tag: Name;
	readonly actions: ReadonlyArray<Actions>;
	readonly options: Options;
}): Actor<Name, Actions> => {
	const key = `@rivetkit/effect/Actor/${options._tag}`;
	return Object.assign(Object.create(Proto), {
		...options,
		key,
	}) as Actor<Name, Actions>;
};

/**
 * Define a Rivet Actor contract.
 */
export const make = <
	const Name extends string,
	const Actions extends ReadonlyArray<Action.AnyWithProps> = readonly [],
>(
	name: Name,
	options?: {
		readonly actions?: Actions;
		readonly options?: Options;
	},
): Actor<Name, Actions[number]> => {
	return makeProto({
		_tag: name,
		actions: (options?.actions ?? []) as ReadonlyArray<Action.AnyWithProps>,
		options: options?.options ?? {},
	}) as any;
};
