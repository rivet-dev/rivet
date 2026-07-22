import type { RunStore } from '@flue/runtime/adapter-kit';

// Rivet actor start/wake is eventually consistent: a freshly-targeted actor can
// briefly report "no envoys"/"no capacity"/wake-retry errors before it is ready.
// These are transient and worth retrying; anything else propagates immediately.
// (Raw HTTP forwarding via `handle.fetch` has its own cold-start retry, so this
// helper is only used for the registry run-store actor calls.)
const TRANSIENT_RIVET_START_ERROR =
	/actor_wake_retries_exceeded|actor_stopped_before_ready|actor_ready_timeout|no_envoys|no_capacity|fetch failed/i;

export function isTransientRivetStartError(error: unknown): boolean {
	return TRANSIENT_RIVET_START_ERROR.test(
		String(error instanceof Error ? error.message : error),
	);
}

export async function retryTransientRivetStart<T>(action: () => Promise<T>): Promise<T> {
	let lastError: unknown;
	for (let attempt = 0; attempt < 6; attempt++) {
		try {
			return await action();
		} catch (error) {
			lastError = error;
			if (!isTransientRivetStartError(error) || attempt === 5) throw error;
			await delay(1_000 * (attempt + 1));
		}
	}
	throw lastError;
}

function delay(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Wrap a Rivet registry actor handle (whose run-store actions mirror the
 * {@link RunStore} surface) as a {@link RunStore}, retrying transient
 * actor-start errors on every call.
 */
export function createRegistryRunStoreFromHandle(handle: RunStore): RunStore {
	return {
		createRun: (input) => retryTransientRivetStart(() => handle.createRun(input)),
		endRun: (input) => retryTransientRivetStart(() => handle.endRun(input)),
		getRun: (runId) => retryTransientRivetStart(() => handle.getRun(runId)),
		lookupRun: (runId) => retryTransientRivetStart(() => handle.lookupRun(runId)),
		listRuns: (opts) => retryTransientRivetStart(() => handle.listRuns(opts)),
	};
}
