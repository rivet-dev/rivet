import type { TracesDriver } from "@rivetkit/traces";
import type { ActorDriver } from "../driver";
import { tracesStoragePrefix } from "./keys";

function concatPrefix(prefix: Uint8Array, key: Uint8Array): Uint8Array {
	const merged = new Uint8Array(prefix.length + key.length);
	merged.set(prefix, 0);
	merged.set(key, prefix.length);
	return merged;
}

function stripPrefix(prefix: Uint8Array, key: Uint8Array): Uint8Array {
	return key.slice(prefix.length);
}

function compareBytes(a: Uint8Array, b: Uint8Array): number {
	const len = Math.min(a.length, b.length);
	for (let i = 0; i < len; i++) {
		if (a[i] !== b[i]) {
			return a[i] - b[i];
		}
	}
	return a.length - b.length;
}

export class ActorTracesDriver implements TracesDriver {
	#driver: ActorDriver;
	#actorId: string;
	#prefix: Uint8Array;

	constructor(driver: ActorDriver, actorId: string) {
		this.#driver = driver;
		this.#actorId = actorId;
		this.#prefix = tracesStoragePrefix();
	}

	async get(key: Uint8Array): Promise<Uint8Array | null> {
		const [value] = await this.#driver.kvBatchGet(this.#actorId, [
			concatPrefix(this.#prefix, key),
		]);
		return value ?? null;
	}

	async set(key: Uint8Array, value: Uint8Array): Promise<void> {
		await this.#driver.kvBatchPut(this.#actorId, [
			[concatPrefix(this.#prefix, key), value],
		]);
	}

	async delete(key: Uint8Array): Promise<void> {
		await this.#driver.kvBatchDelete(this.#actorId, [
			concatPrefix(this.#prefix, key),
		]);
	}

	async deletePrefix(prefix: Uint8Array): Promise<void> {
		const fullPrefix = concatPrefix(this.#prefix, prefix);
		const entries = await this.#driver.kvListPrefix(
			this.#actorId,
			fullPrefix,
		);
		if (entries.length === 0) {
			return;
		}
		await this.#driver.kvBatchDelete(
			this.#actorId,
			entries.map(([key]) => key),
		);
	}

	async list(
		prefix: Uint8Array,
	): Promise<Array<{ key: Uint8Array; value: Uint8Array }>> {
		const fullPrefix = concatPrefix(this.#prefix, prefix);
		const entries = await this.#driver.kvListPrefix(
			this.#actorId,
			fullPrefix,
		);
		return entries.map(([key, value]) => ({
			key: stripPrefix(this.#prefix, key),
			value,
		}));
	}

	async listRange(
		start: Uint8Array,
		end: Uint8Array,
		options?: { reverse?: boolean; limit?: number },
	): Promise<Array<{ key: Uint8Array; value: Uint8Array }>> {
		const fullStart = concatPrefix(this.#prefix, start);
		const fullEnd = concatPrefix(this.#prefix, end);
		const entries = await this.#driver.kvListPrefix(
			this.#actorId,
			this.#prefix,
		);
		const filtered = entries
			.filter(([key]) => {
				return (
					compareBytes(key, fullStart) >= 0 &&
					compareBytes(key, fullEnd) < 0
				);
			})
			.sort(([keyA], [keyB]) => compareBytes(keyA, keyB));
		if (options?.reverse) {
			filtered.reverse();
		}
		const limited = options?.limit
			? filtered.slice(0, options.limit)
			: filtered;
		return limited.map(([key, value]) => ({
			key: stripPrefix(this.#prefix, key),
			value,
		}));
	}

	async batch(writes: Array<{ key: Uint8Array; value: Uint8Array }>): Promise<void> {
		if (writes.length === 0) {
			return;
		}
		await this.#driver.kvBatchPut(
			this.#actorId,
			writes.map(({ key, value }) => [
				concatPrefix(this.#prefix, key),
				value,
			]),
		);
	}
}
