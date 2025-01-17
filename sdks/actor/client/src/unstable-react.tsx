"use client";
// @ts-ignore: This is a temporary workaround until the React Server Components will be fully released.
import { createFromFetch } from "@kentcdodds/tmp-react-server-dom-esm/client.browser";
// deno-lint-ignore verbatim-module-syntax
import type * as React from "react";
import {
	type ElementType,
	createContext,
	use,
	useCallback,
	useEffect,
	useMemo,
	useRef,
	useState,
	useSyncExternalStore,
} from "react";
import type { Client as ActorClient, ActorHandle } from "./client.ts";

export const ActorClientContext = createContext<ActorClient | null>(null);

export function ActorClientProvider({
	client,
	children,
}: {
	children: React.ReactNode;
	client: ActorClient;
}) {
	return (
		<ActorClientContext.Provider value={client}>
			{children}
		</ActorClientContext.Provider>
	);
}

export function useActorClient() {
	const manager = use(ActorClientContext);
	if (!manager) {
		throw new Error(
			"useActorClient must be used within an ActorClientProvider",
		);
	}
	return manager;
}

const noop = () => {
	// noop
};

/**
 * Shallow compare objects.
 * Copied from https://github.com/TanStack/query/blob/3c5d8e348cc53e46aea6c74767f3181fc77c2308/packages/query-core/src/utils.ts#L298-L299
 */
export function shallowEqualObjects<
	// biome-ignore lint/suspicious/noExplicitAny: we do not care about the shape
	T extends Record<string, any>,
>(a: T | undefined, b: T | undefined): boolean {
	if (a === undefined && b === undefined) {
		return true;
	}
	if (!a || !b || Object.keys(a).length !== Object.keys(b).length) {
		return false;
	}

	for (const key in a) {
		if (a[key] !== b[key]) {
			if (typeof a[key] === "object" && typeof b[key] === "object") {
				return shallowEqualObjects(a[key], b[key]);
			}
			return false;
		}
	}

	return true;
}

const ACTOR_HANDLE_STATE_INIT = {
	isLoading: false,
	error: null,
	actor: null,
} as const;
const ACTOR_HANDLE_STATE_CREATING = {
	isLoading: true,
	error: null,
	actor: null,
} as const;
function ACTOR_HANDLE_STATE_CREATED<A = unknown>(actor: ActorHandle<A>) {
	return { isLoading: false, error: null, actor } as const;
}
function ACTOR_HANDLE_STATE_ERRORED(error: unknown) {
	return { isLoading: false, error, actor: null } as const;
}

type ActorManagerState<A> =
	| typeof ACTOR_HANDLE_STATE_INIT
	| typeof ACTOR_HANDLE_STATE_CREATING
	| { isLoading: false; error: null; actor: ActorHandle<A> }
	| { isLoading: false; error: unknown; actor: null };

class ActorManager<A = unknown> {
	#client: ActorClient;
	#options: Parameters<ActorClient["get"]>;

	#listeners: (() => void)[] = [];

	#state: ActorManagerState<A> = ACTOR_HANDLE_STATE_INIT;

	#createPromise: Promise<ActorHandle<A>> | null = null;

	constructor(client: ActorClient, options: Parameters<ActorClient["get"]>) {
		this.#client = client;
		this.#options = options;
	}

	setOptions(options: Parameters<ActorClient["get"]>) {
		if (shallowEqualObjects(options, this.#options)) {
			if (!this.#state.actor) {
				this.create();
			}
			return;
		}

		this.#state.actor?.dispose();

		this.#state = ACTOR_HANDLE_STATE_INIT;
		this.#options = options;
		this.#update();
		this.create();
	}

	async create() {
		if (this.#createPromise) {
			return this.#createPromise;
		}
		this.#state = { ...ACTOR_HANDLE_STATE_CREATING };
		this.#update();
		try {
			this.#createPromise = this.#client.get<A>(...this.#options);
			const actor = await this.#createPromise;
			this.#state = ACTOR_HANDLE_STATE_CREATED(actor);
			this.#createPromise = null;
		} catch (e) {
			this.#state = ACTOR_HANDLE_STATE_ERRORED(e);
		} finally {
			this.#update();
		}
	}

	getState() {
		return this.#state;
	}

	subscribe(cb: () => void) {
		this.#listeners.push(cb);
		return () => {
			this.#listeners = this.#listeners.filter((l) => l !== cb);
		};
	}

	#update() {
		for (const cb of this.#listeners) {
			cb();
		}
	}
}

function useActorHook<A = unknown>(...options: Parameters<ActorClient["get"]>) {
	const client = useActorClient();

	const [manager] = useState(() => new ActorManager<A>(client, options));

	const state = useSyncExternalStore(
		useCallback(
			(onUpdate) => {
				return manager.subscribe(onUpdate);
			},
			[manager],
		),
		() => manager.getState(),
		() => manager.getState(),
	);

	useEffect(() => {
		manager.setOptions(options);
	}, [options, manager]);

	return [state] as const;
}

function useActorEventHook<
	A = unknown,
	// biome-ignore lint/suspicious/noExplicitAny: we do not care about the shape of the args, for now
	Args extends any[] = unknown[],
>(
	opts: { actor: ActorHandle<A> | null; event: string },
	cb: (...args: Args) => void,
) {
	const ref = useRef(cb);

	useEffect(() => {
		ref.current = cb;
	}, [cb]);

	useEffect(() => {
		if (!opts.actor) {
			return noop;
		}
		const unsub = opts.actor.on(opts.event, (...args: Args) => {
			ref.current(...args);
		});

		return unsub;
	}, [opts.actor, opts.event]);
}

function useRscHook<A = unknown>(opts: {
	actor: ActorHandle<A> | null;
	fn: keyof ActorHandle<A>;
}) {
	const [lastResponse, setLastResponse] = useState<ElementType>(
		() => () => null,
	);

	const rsc = useCallback(
		async (
			// biome-ignore lint/suspicious/noExplicitAny: we do not care about the shape of the args, for now
			...args: any[]
		) => {
			if (opts.actor === null) {
				return () => null;
			}
			const response = (await opts.actor[opts.fn](...args)) as string;
			const data = new Response(response);
			const jsx = await createFromFetch(Promise.resolve(data));
			setLastResponse(() => () => jsx);

			return jsx;
		},
		[opts.actor, opts.fn],
	);

	return [lastResponse, rsc] as const;
}

type UseActorParameters =
	| [
			Omit<Parameters<ActorClient["get"]>[0], "name">,
			Parameters<ActorClient["get"]>[1],
	  ]
	| [Omit<Parameters<ActorClient["get"]>[0], "name">]
	| [];

export function unstable_createActorHooks<A = unknown>(opts: { name: string }) {
	return {
		useActor(...options: UseActorParameters) {
			const [req, ...rest] = options;
			return useActorHook<A>({ ...opts, ...req }, ...rest);
		},
		useActorEventCallback<
			E extends string,
			// biome-ignore lint/suspicious/noExplicitAny: we do not care about the shape of the args, for now
			Args extends any[] = unknown[],
		>(
			opts: { actor: ActorHandle<A> | null; event: E },
			cb: (...args: Args) => void,
		) {
			return useActorEventHook<A, Args>(opts, cb);
		},
		useActorRsc<A = unknown>(opts: {
			actor: ActorHandle<A> | null;
			fn: keyof ActorHandle<A>;
		}) {
			return useRscHook(opts);
		},
	};
}

function ServerComponent<A>({
	actor,
	fn,
	...props
}: {
	actor: ActorHandle<A> | null;
	fn: keyof ActorHandle<A>;
}) {
	const [Comp, refetch] = useRscHook({ actor, fn });
	useActorEventHook<A>({ actor, event: "__rsc" }, () => {
		refetch(props);
	});
	// biome-ignore lint/correctness/useExhaustiveDependencies: We don't want to refetch on every render, only on mount
	useEffect(() => {
		refetch(props);
	}, []);

	return <Comp />;
}

export function useActor<A = unknown>(
	...options: Parameters<ActorClient["get"]>
) {
	const [{ actor, ...rest }] = useActorHook<A>(...options);

	const rsc = useMemo(() => {
		return new Proxy({} as ActorHandle<A>, {
			get(_target, prop) {
				// biome-ignore lint/suspicious/noExplicitAny: we do not care about the shape of the args, for now
				return (props: Record<string, any>) => (
					<ServerComponent<A>
						actor={actor}
						fn={prop as keyof ActorHandle<A>}
						{...props}
					/>
				);
			},
		});
	}, [actor]);

	return [{ actor, ...rest }, rsc] as const;
}
