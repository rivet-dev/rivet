import { CancelledError } from "@tanstack/react-query";
import ActorWorker from "./actor-repl.worker?worker";
import {
	type CodeMessage,
	type FormattedCode,
	type InitMessage,
	type Log,
	ResponseSchema,
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
	commands: ReplCommand[];
};

export class ActorWorkerContainer {
	#state: ContainerState = {
		status: { type: "unknown" },
		commands: [],
	};

	#meta: {
		actorId: string;
		rpcs: string[];
		endpoint?: string;
		name?: string;
	} | null = null;

	#listeners: (() => void)[] = [];
	#worker: Worker | undefined;

	//
	async init({
		actorId,
		signal,
		rpcs = [],
		endpoint,
		name,
	}: {
		actorId: string;
		signal: AbortSignal;
		rpcs?: string[];
		endpoint?: string;
		name?: string;
	}) {
		this.terminate();

		this.#meta = { actorId, rpcs, endpoint, name };
		this.#state.status = { type: "pending" };
		this.#update();
		try {
			signal.throwIfAborted();

			// FIXME(RVT-4553)
			// if (actor.resources.cpu !== 125 || actor.resources.memory !== 128) {
			// 	throw new Error("Unsupported actor resources");
			// }

			// If we reached this point, the actor is supported
			// check if we still operate on the same actor
			if (this.#meta.actorId !== actorId) {
				// if not, we don't need to do anything
				return null;
			}

			const worker = new ActorWorker({ name: `actor-${actorId}` });
			signal.throwIfAborted();
			// now worker needs to check if the actor is supported
			this.#setupWorker(worker);
			signal.throwIfAborted();
			return worker;
		} catch (e) {
			console.log(e);
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
		this.#meta = null;
		this.#update();
	}

	#setupWorker(worker: Worker) {
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
			rpcs: this.#meta?.rpcs ?? [],
			id: this.#meta?.actorId ?? "",
			endpoint: this.#meta?.endpoint ?? "",
			name: this.#meta?.name ?? "",
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

	getCommands() {
		return this.#state.commands;
	}

	getStatus() {
		return this.#state.status;
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
			this.#update();
		}
	}

	#update() {
		for (const listener of this.#listeners) {
			listener();
		}
	}
}
