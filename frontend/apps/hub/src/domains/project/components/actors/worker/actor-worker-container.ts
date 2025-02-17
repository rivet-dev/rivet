import { actorQueryOptions } from "@/domains/project/queries";
import { queryClient } from "@/queries/global";
import { CancelledError } from "@tanstack/react-query";
import ActorWorker from "./actor-repl.worker?worker";
import {
	type State as ActorState,
	type CodeMessage,
	type Connection,
	type FormattedCode,
	type InitMessage,
	type Log,
	ResponseSchema,
	type SetStateMessage,
} from "./actor-worker-schema";

export type ReplCommand = {
	logs: Log[];
	code: string;
	key: string;
	inputTimestamp?: string;
	outputTimestamp?: string;
} & (
	| { status: "pending" }
	| { status: "formatted"; formatted: FormattedCode }
	| { status: "success"; formatted: FormattedCode; result: unknown }
	| { status: "error"; formatted: FormattedCode | undefined; error: unknown }
);

export type ContainerStatus =
	| { type: "ready" }
	| { type: "error"; error: unknown }
	| { type: "pending" }
	| { type: "unsupported"; error: unknown }
	| { type: "unknown" };

export type ContainerState = {
	status: ContainerStatus;
	rpcs: string[];
	commands: ReplCommand[];
	state: ActorState & { json: unknown };
	connections: Connection[];
};

export class ActorWorkerContainer {
	#state: ContainerState = {
		status: { type: "unknown" },
		commands: [],
		rpcs: [],
		state: { enabled: false, native: undefined, json: undefined },
		connections: [],
	};

	#meta: {
		projectNameId: string;
		environmentNameId: string;
		actorId: string;
	} | null = null;

	#listeners: (() => void)[] = [];
	#worker: Worker | undefined;

	//
	async init({
		projectNameId,
		environmentNameId,
		actorId,
		endpoint,
		signal,
	}: {
		projectNameId: string;
		environmentNameId: string;
		actorId: string;
		endpoint: string;
		signal: AbortSignal;
	}) {
		this.terminate();

		this.#meta = { projectNameId, environmentNameId, actorId };
		this.#state.status = { type: "pending" };
		this.#update();
		try {
			// If we have the manager URL, next we need to check actor's runtime
			const { actor } = await queryClient.fetchQuery(
				actorQueryOptions({
					projectNameId,
					environmentNameId,
					actorId,
				}),
			);
			signal.throwIfAborted();

			// FIXME(RVT-4553)
			if (actor.resources.cpu !== 125 || actor.resources.memory !== 128) {
				throw new Error("Unsupported actor resources");
			}

			// If we reached this point, the actor is supported
			// check if we still operate on the same actor
			if (
				this.#meta.projectNameId !== projectNameId ||
				this.#meta.environmentNameId !== environmentNameId ||
				this.#meta.actorId !== actorId
			) {
				// if not, we don't need to do anything
				return null;
			}

			const worker = new ActorWorker({ name: `actor-${actorId}` });
			signal.throwIfAborted();
			// now worker needs to check if the actor is supported
			this.#setupWorker(worker, { actorId, endpoint });
			signal.throwIfAborted();
			return worker;
		} catch (e) {
			// If we reached this point, the actor is not supported
			// check if we still operate on the same actor
			if (e instanceof DOMException && e.name === "AbortError") {
				return null;
			}

			if (e instanceof CancelledError) {
				this.#worker?.terminate();
				this.#worker = undefined;
				return null;
			}

			this.#worker?.terminate();
			this.#worker = undefined;
			this.#state.status = { type: "unsupported", error: e };
			this.#update();
		}
		return null;
	}

	terminate() {
		this.#worker?.terminate();
		this.#worker = undefined;
		this.#state.commands = [];
		this.#state.status = { type: "unknown" };
		this.#state.rpcs = [];
		this.#state.state = {
			enabled: false,
			native: undefined,
			json: undefined,
		};
		this.#meta = null;
		this.#state.connections = [];
		this.#update();
	}

	#setupWorker(worker: Worker, data: Omit<InitMessage, "type">) {
		this.#worker = worker;
		this.#worker.addEventListener("message", (event) => {
			try {
				this.#handleMessage(event);
			} catch (e) {
				console.error(e);
				this.#state.status = { type: "error", error: e };
				this.#update();
			}
		});

		this.#worker.addEventListener("error", (error) => {
			console.log(error, error.message, error.error);
		});

		this.#worker.postMessage({
			type: "init",
			...data,
		} satisfies InitMessage);
	}

	run(data: string) {
		const key = Date.now().toString();
		this.#state.commands = [
			...this.#state.commands,
			{ status: "pending", code: data, key, logs: [] },
		];

		this.#worker?.postMessage({
			type: "code",
			data,
			id: key,
		} satisfies CodeMessage);
		this.#update();
	}

	setState(data: string) {
		this.#worker?.postMessage({
			type: "set-state",
			data,
		} satisfies SetStateMessage);
		this.#state.state = {
			...this.#state.state,
			native: data,
			json: JSON.parse(data || "{}"),
		};
		this.#update();
	}

	getCommands() {
		return this.#state.commands;
	}

	getStatus() {
		return this.#state.status;
	}

	getRpcs() {
		return this.#state.rpcs;
	}

	getState() {
		return this.#state.state;
	}

	getConnections() {
		return this.#state.connections;
	}

	subscribe(cb: () => void) {
		this.#listeners.push(cb);
		return () => {
			this.#listeners = this.#listeners.filter(
				(listener) => listener !== cb,
			);
		};
	}

	#handleMessage(event: MessageEvent) {
		const { success, data: msg } = ResponseSchema.safeParse(event.data);

		if (!success) {
			return;
		}

		if (msg.type === "formatted") {
			const command = this.#state.commands.find(
				(command) => command.key === msg.id,
			);
			if (command) {
				const newCommand = {
					inputTimestamp: new Date().toISOString(),
					...command,
					status: "formatted",
					formatted: msg.data,
				} satisfies ReplCommand;
				Object.assign(command, newCommand);
				this.#state.commands = [...this.#state.commands];
				this.#update();
			}
		}

		if (msg.type === "result") {
			const command = this.#state.commands.find(
				(command) => command.key === msg.id,
			);
			if (command) {
				const newCommand = {
					outputTimestamp: new Date().toISOString(),
					...command,
					status: "success",
					result: msg.data,
				};
				Object.assign(command, newCommand);
				this.#state.commands = [...this.#state.commands];
				this.#update();
			}
		}

		if (msg.type === "log") {
			const command = this.#state.commands.find(
				(command) => command.key === msg.id,
			);
			if (command) {
				const newCommand = {
					...command,
					logs: [...command.logs, msg.data],
				};
				Object.assign(command, newCommand);
				this.#state.commands = [...this.#state.commands];
				this.#update();
			}
		}

		if (msg.type === "error") {
			if (!msg.id) {
				this.#state.status = { type: "error", error: msg.data };
				console.error("Actor Worker Error", msg.data);
				this.#update();
				return;
			}

			const command = this.#state.commands.find(
				(command) => command.key === msg.id,
			);
			if (command) {
				const newCommand = {
					outputTimestamp: new Date().toISOString(),
					...command,
					status: "error",
					error: msg.data,
				};
				Object.assign(command, newCommand);
				this.#state.commands = [...this.#state.commands];
				this.#update();
			}
		}

		if (msg.type === "ready") {
			this.#state.status = { type: "ready" };
			this.#state.rpcs = msg.data.rpcs;
			this.#state.state = {
				...msg.data.state,
				json: JSON.parse(msg.data.state.native),
			};
			this.#state.connections = msg.data.connections;
			this.#update();
		}

		if (msg.type === "state-change") {
			this.#state.state = {
				...msg.data,
				native: msg.data.native,
				json: JSON.parse(msg.data.native || "{}"),
			};
			this.#update();
		}

		if (msg.type === "connections-change") {
			this.#state.connections = [...msg.data];
			this.#update();
		}
	}

	#update() {
		for (const listener of this.#listeners) {
			listener();
		}
	}
}
