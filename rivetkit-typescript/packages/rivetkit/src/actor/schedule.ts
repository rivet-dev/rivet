import type { AnyStaticActorInstance } from "./instance/mod";

export class Schedule {
	#actor: AnyStaticActorInstance;

	constructor(actor: AnyStaticActorInstance) {
		this.#actor = actor;
	}

	async after(duration: number, fn: string, ...args: unknown[]) {
		await this.#actor.scheduleEvent(Date.now() + duration, fn, args);
	}

	async at(timestamp: number, fn: string, ...args: unknown[]) {
		await this.#actor.scheduleEvent(timestamp, fn, args);
	}
}
