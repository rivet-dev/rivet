import { ActorError } from "./errors";

/** How many retries a client may spend, as a share of the requests it sends. */
export interface RetryBudgetOptions {
	/** Retries allowed per 100 requests (default 10). */
	percent?: number;
	/** Retries the client can bank for a burst of refusals; the budget starts full (default 100). */
	maxTokens?: number;
}

const DEFAULT_PERCENT = 10;
const DEFAULT_MAX_TOKENS = 100;

/**
 * Bounds the client's retries of refused requests. Each request adds a fraction of a token and
 * each retry spends one, so retries stay a fixed share of traffic while the service sheds load
 * instead of multiplying it.
 */
export class RetryBudget {
	#tokens: number;
	readonly #maxTokens: number;
	readonly #deposit: number;

	constructor(options: RetryBudgetOptions = {}) {
		const percent = options.percent ?? DEFAULT_PERCENT;
		const maxTokens = options.maxTokens ?? DEFAULT_MAX_TOKENS;
		if (!(percent >= 0 && percent <= 100)) {
			throw new Error("retryBudget.percent must be in 0..=100");
		}
		if (!(maxTokens >= 0)) {
			throw new Error("retryBudget.maxTokens must be at least 0");
		}
		this.#maxTokens = maxTokens;
		this.#tokens = maxTokens;
		this.#deposit = percent / 100;
	}

	/** Credits one request sent. */
	deposit(): void {
		this.#tokens = Math.min(this.#maxTokens, this.#tokens + this.#deposit);
	}

	/** Spends one token. Returns `false` when the budget is spent. */
	tryWithdraw(): boolean {
		// Rounding guards against float drift: ten 0.1 deposits must make one token.
		if (this.#tokens + 1e-9 < 1) {
			return false;
		}
		this.#tokens = Math.max(0, this.#tokens - 1);
		return true;
	}
}

/** The service's retry hint for a refused request, in milliseconds, when it sent one. */
export function retryAfterMs(error: unknown): number | undefined {
	if (!(error instanceof ActorError)) {
		return undefined;
	}
	const metadata = error.metadata;
	if (metadata === null || typeof metadata !== "object") {
		return undefined;
	}
	const hint = (metadata as { retry_after_ms?: unknown }).retry_after_ms;
	return typeof hint === "number" && hint >= 0 ? hint : undefined;
}

/**
 * The wait before retry `attempt` (0-based): jittered exponential backoff over
 * `[initialDelayMs, maxDelayMs]`, never shorter than the service's hint.
 */
export function retryDelayMs(
	attempt: number,
	opts: { initialDelayMs: number; maxDelayMs: number; hintMs?: number },
): number {
	const backoff = Math.min(opts.initialDelayMs * 2 ** attempt, opts.maxDelayMs);
	const jittered = backoff / 2 + Math.random() * (backoff / 2);
	return Math.max(jittered, opts.hintMs ?? 0);
}
