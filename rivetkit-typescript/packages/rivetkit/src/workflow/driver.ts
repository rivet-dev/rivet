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

const WORKFLOW_QUEUE_PREFIX = "__workflow:";

export function workflowQueueName(name: string): string {
	return `${WORKFLOW_QUEUE_PREFIX}${name}`;
}

function stripWorkflowQueueName(name: string): string | null {
	if (!name.startsWith(WORKFLOW_QUEUE_PREFIX)) {
		return null;
	}
	return name.slice(WORKFLOW_QUEUE_PREFIX.length);
}

function stripWorkflowKey(prefixed: Uint8Array): Uint8Array {
	return prefixed.slice(KEYS.WORKFLOW_PREFIX.length);
}

class ActorWorkflowMessageDriver implements WorkflowMessageDriver {
	#actor: AnyActorInstance;
	#runCtx: RunContext<any, any, any, any, any, any>;
	#completionHandles = new Map<string, (response?: unknown) => Promise<void>>();

	constructor(
		actor: AnyActorInstance,
		runCtx: RunContext<any, any, any, any, any, any>,
	) {
		this.#actor = actor;
		this.#runCtx = runCtx;
	}

	async loadMessages(): Promise<Message[]> {
		const queueMessages = await this.#runCtx.keepAwake(
			this.#actor.queueManager.getMessages(),
		);

		const workflowMessages: Message[] = [];
		for (const queueMessage of queueMessages) {
			const workflowName = stripWorkflowQueueName(queueMessage.name);
			if (!workflowName) continue;
			const id = queueMessage.id.toString();
			this.#completionHandles.set(id, async (response?: unknown) => {
				await this.#runCtx.keepAwake(
					this.#actor.queueManager.completeMessage(queueMessage, response),
				);
			});
			workflowMessages.push({
				id,
				name: workflowName,
				data: queueMessage.body,
				sentAt: queueMessage.createdAt,
				complete: async (response?: unknown) => {
					await this.completeMessage(id, response);
				},
			});
		}

		return workflowMessages;
	}

	async addMessage(message: Message): Promise<void> {
		await this.#runCtx.keepAwake(
			this.#actor.queueManager.enqueue(
				workflowQueueName(message.name),
				message.data,
			),
		);
	}

	async deleteMessages(messageIds: string[]): Promise<string[]> {
		if (messageIds.length === 0) {
			return [];
		}

		const ids = messageIds.map((id) => {
			try {
				return BigInt(id);
			} catch {
				return null;
			}
		});

		const validIds = ids.filter(
			(id): id is bigint => id !== null && id >= 0,
		);
		if (validIds.length === 0) {
			return [];
		}

		const deleted = await this.#runCtx.keepAwake(
			this.#actor.queueManager.deleteMessagesById(validIds),
		);

		return deleted.map((id) => id.toString());
	}

	async completeMessage(messageId: string, response?: unknown): Promise<void> {
		const complete = this.#completionHandles.get(messageId);
		if (complete) {
			await complete(response);
			this.#completionHandles.delete(messageId);
			return;
		}

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
	#runCtx: RunContext<any, any, any, any, any, any>;

	constructor(
		actor: AnyActorInstance,
		runCtx: RunContext<any, any, any, any, any, any>,
	) {
		this.#actor = actor;
		this.#runCtx = runCtx;
		this.messageDriver = new ActorWorkflowMessageDriver(actor, runCtx);
	}

	#log(msg: string, data?: Record<string, unknown>) {
		this.#runCtx.log.info({ msg: `[workflow-driver] ${msg}`, ...data });
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
		const queueNames = messageNames.map((name) => workflowQueueName(name));
		return this.#actor.queueManager.waitForNames(queueNames, abortSignal);
	}
}
