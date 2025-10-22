import type { AnyActorInstance } from "./instance";

export class Schedule {
	#actor: AnyActorInstance;

	constructor(actor: AnyActorInstance) {
		this.#actor = actor;
	}

	async after(duration: number, fn: string, ...args: unknown[]) {
		await this.#actor.scheduleEvent(Date.now() + duration, fn, args);
	}

	async at(timestamp: number, fn: string, ...args: unknown[]) {
		await this.#actor.scheduleEvent(timestamp, fn, args);
	}
}
