import type { RunContext } from "@/actor/contexts/run";
import type { AnyActorInstance } from "@/actor/instance/mod";
import { KEYS, makeWorkflowKey } from "@/actor/instance/keys";
import type {
	EngineDriver,
	KVEntry,
	KVWrite,
	Message,
	WorkflowMessageDriver,
} from "@rivetkit/workflow-engine";

function stripWorkflowKey(prefixed: Uint8Array): Uint8Array {
	return prefixed.slice(KEYS.WORKFLOW_PREFIX.length);
}

class ActorWorkflowMessageDriver implements WorkflowMessageDriver {
	#actor: AnyActorInstance;
	#runCtx: RunContext<any, any, any, any, any, any, any, any>;

	constructor(
		actor: AnyActorInstance,
		runCtx: RunContext<any, any, any, any, any, any, any, any>,
	) {
		this.#actor = actor;
		this.#runCtx = runCtx;
	}

	async addMessage(message: Message): Promise<void> {
		await this.#runCtx.keepAwake(
			this.#actor.queueManager.enqueue(message.name, message.data),
		);
	}

	async receiveMessages(opts: {
		names?: readonly string[];
		count: number;
		completable: boolean;
	}): Promise<Message[]> {
		const messages = await this.#runCtx.keepAwake(
			this.#actor.queueManager.receive(
				opts.names && opts.names.length > 0 ? [...opts.names] : undefined,
				opts.count,
				0,
				undefined,
				opts.completable,
			),
		);
		return messages.map((message) => ({
			id: message.id.toString(),
			name: message.name,
			data: message.body,
			sentAt: message.createdAt,
			...(opts.completable
				? {
						complete: async (response?: unknown) => {
							await this.#runCtx.keepAwake(
								this.#actor.queueManager.completeMessage(
									message,
									response,
								),
							);
						},
					}
				: {}),
		}));
	}

	async completeMessage(messageId: string, response?: unknown): Promise<void> {
		let parsedId: bigint;
		try {
			parsedId = BigInt(messageId);
		} catch {
			return;
		}

		await this.#runCtx.keepAwake(
			this.#actor.queueManager.completeMessageById(parsedId, response),
		);
	}
}

export class ActorWorkflowDriver implements EngineDriver {
	readonly workerPollInterval = 100;
	readonly messageDriver: WorkflowMessageDriver;
	#actor: AnyActorInstance;
	#runCtx: RunContext<any, any, any, any, any, any, any, any>;

	constructor(
		actor: AnyActorInstance,
		runCtx: RunContext<any, any, any, any, any, any, any, any>,
	) {
		this.#actor = actor;
		this.#runCtx = runCtx;
		this.messageDriver = new ActorWorkflowMessageDriver(actor, runCtx);
	}

	async get(key: Uint8Array): Promise<Uint8Array | null> {
		const [value] = await this.#runCtx.keepAwake(
			this.#actor.driver.kvBatchGet(this.#actor.id, [
				makeWorkflowKey(key),
			]),
		);
		return value ?? null;
	}

	async set(key: Uint8Array, value: Uint8Array): Promise<void> {
		await this.#runCtx.keepAwake(
			this.#actor.driver.kvBatchPut(this.#actor.id, [
				[makeWorkflowKey(key), value],
			]),
		);
	}

	async delete(key: Uint8Array): Promise<void> {
		await this.#runCtx.keepAwake(
			this.#actor.driver.kvBatchDelete(this.#actor.id, [
				makeWorkflowKey(key),
			]),
		);
	}

	async deletePrefix(prefix: Uint8Array): Promise<void> {
		const entries = await this.#runCtx.keepAwake(
			this.#actor.driver.kvListPrefix(
				this.#actor.id,
				makeWorkflowKey(prefix),
			),
		);
		if (entries.length === 0) {
			return;
		}
		await this.#runCtx.keepAwake(
			this.#actor.driver.kvBatchDelete(
				this.#actor.id,
				entries.map(([key]) => key),
			),
		);
	}

	async list(prefix: Uint8Array): Promise<KVEntry[]> {
		const entries = await this.#runCtx.keepAwake(
			this.#actor.driver.kvListPrefix(
				this.#actor.id,
				makeWorkflowKey(prefix),
			),
		);
		return entries.map(([key, value]) => ({
			key: stripWorkflowKey(key),
			value,
		}));
	}

	async batch(writes: KVWrite[]): Promise<void> {
		if (writes.length === 0) return;

		// Flush actor state together with workflow state to ensure atomicity.
		// If the server crashes after workflow flush, actor state must also be persisted.
		await this.#runCtx.keepAwake(
			Promise.all([
				this.#actor.driver.kvBatchPut(
					this.#actor.id,
					writes.map(({ key, value }) => [makeWorkflowKey(key), value]),
				),
				this.#actor.stateManager.saveState({ immediate: true }),
			]),
		);
	}

	async setAlarm(_workflowId: string, wakeAt: number): Promise<void> {
		await this.#runCtx.keepAwake(
			this.#actor.driver.setAlarm(this.#actor, wakeAt),
		);
	}

	async clearAlarm(_workflowId: string): Promise<void> {
		// No dedicated clear alarm support in actor drivers.
		return;
	}

	waitForMessages(
		messageNames: string[],
		abortSignal: AbortSignal,
	): Promise<void> {
		return this.#actor.queueManager.waitForNames(
			messageNames.length > 0 ? messageNames : undefined,
			abortSignal,
		);
	}
}
