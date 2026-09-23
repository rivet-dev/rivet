/** Called when RivetKit needs a credential. Keep issuer secrets on your backend. */
export type GetToken = (options: { forceRefresh: boolean }) => Promise<string>;

/** Only a confirmed invalid/expired credential can trigger an automatic renewal. */
export function isInvalidToken(group: string, code: string): boolean {
	return (
		group === "auth" &&
		(code === "invalid_token" || code === "token_expired")
	);
}

export function isInvalidTokenResponse(response: Response): boolean {
	if (response.status !== 401) return false;
	const error = response.headers.get("x-rivet-error");
	if (!error) return false;
	const separator = error.indexOf(".");
	return (
		separator > 0 &&
		isInvalidToken(error.slice(0, separator), error.slice(separator + 1))
	);
}

/** One token cache per client; concurrent requests and refreshes share the same issuance call. */
export class TokenProvider {
	#cached?: { token: string; expiresAt?: number };
	#pending?: Promise<string>;
	#pendingForce = false;

	constructor(private readonly getToken: GetToken) {}

	async current(): Promise<string> {
		if (
			this.#cached &&
			(this.#cached.expiresAt === undefined ||
				Date.now() < this.#cached.expiresAt - 5_000)
		) {
			return this.#cached.token;
		}
		return this.#load(false);
	}

	/** A late rejection for an older token must never erase a newer credential. */
	async refreshIfCurrent(failedToken: string): Promise<string> {
		if (this.#pending !== undefined) {
			const wasForced = this.#pendingForce;
			const refreshed = await this.#pending;
			if (wasForced || refreshed !== failedToken) return refreshed;
		}
		if (this.#cached?.token !== failedToken) {
			return this.current();
		}
		return this.#load(true);
	}

	#load(forceRefresh: boolean): Promise<string> {
		if (this.#pending !== undefined) return this.#pending;

		const pending = Promise.resolve()
			.then(() => this.getToken({ forceRefresh }))
			.then((token) => {
				if (typeof token !== "string" || token.length === 0) {
					throw new Error(
						"getToken must resolve to a nonempty string",
					);
				}
				this.#cached = { token, expiresAt: jwtExpiration(token) };
				return token;
			})
			.finally(() => {
				if (this.#pending === pending) this.#pending = undefined;
			});
		this.#pending = pending;
		this.#pendingForce = forceRefresh;
		return pending;
	}
}

// This is only a cache hint, never a signature or authorization check. The Engine verifies JWTs.
function jwtExpiration(token: string): number | undefined {
	try {
		const payload = token.split(".")[1];
		if (!payload) return undefined;
		const claims = JSON.parse(
			atob(payload.replace(/-/g, "+").replace(/_/g, "/")),
		) as { exp?: unknown };
		return typeof claims.exp === "number" &&
			Number.isSafeInteger(claims.exp)
			? claims.exp * 1_000
			: undefined;
	} catch {
		return undefined;
	}
}
