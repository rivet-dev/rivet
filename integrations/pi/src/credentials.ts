import type { PiCredential, PiCredentialStore } from "./models.js";

/**
 * A provider credential an application supplies. A subscription login never
 * includes its refresh token, because the application refreshes it.
 */
export type PiProviderCredential =
	| Extract<PiCredential, { type: "api_key" }>
	| { type: "oauth"; access: string; expires: number; [field: string]: unknown };

/** A provider that has a credential, without the secret. */
export interface PiCredentialInfo {
	providerId: string;
	type: PiProviderCredential["type"];
}

/**
 * Provider credentials an application supplies to a Pi actor. The application
 * owns login, storage, and refresh. Errors reject the model call that needed
 * the credential, so their messages must not contain secrets.
 */
export interface PiCredentialSource {
	/** Providers that have a credential. */
	list(): Promise<PiCredentialInfo[]>;
	/** A provider's credential, possibly expired. */
	read(providerId: string): Promise<PiProviderCredential | undefined>;
	/**
	 * A provider's credential after the application refreshed it. Pi calls this
	 * when a subscription token expires within five minutes, so the result must
	 * be valid for longer than that.
	 */
	refresh(providerId: string): Promise<PiProviderCredential | undefined>;
}

/** How long Pi requires an OAuth token to stay valid before it refreshes it. */
const PI_OAUTH_MIN_VALIDITY_MS = 5 * 60_000;

/**
 * Adapts an application's credential source to Pi's credential store. It
 * reads only providers the source lists, keeps what it read until
 * `invalidate` runs, and never writes.
 */
export class SourceCredentialStore implements PiCredentialStore {
	readonly #source: PiCredentialSource;
	#listed: Promise<PiCredentialInfo[]> | undefined;
	readonly #read = new Map<string, Promise<PiCredential | undefined>>();

	constructor(source: PiCredentialSource) {
		this.#source = source;
	}

	/** Drops what was read so the next read sees new logins and logouts. */
	invalidate(): void {
		this.#listed = undefined;
		this.#read.clear();
	}

	list(): Promise<PiCredentialInfo[]> {
		this.#listed ??= this.#source.list().catch((error: unknown) => {
			this.#listed = undefined;
			throw error;
		});
		return this.#listed;
	}

	async read(providerId: string): Promise<PiCredential | undefined> {
		if (!(await this.list()).some((entry) => entry.providerId === providerId)) return undefined;
		let read = this.#read.get(providerId);
		if (!read) {
			read = this.#source
				.read(providerId)
				.then(toPiCredential)
				.catch((error: unknown) => {
					this.#read.delete(providerId);
					throw error;
				});
			this.#read.set(providerId, read);
		}
		return read;
	}

	/**
	 * Pi calls this to refresh a subscription token that expires soon. The
	 * source refreshes it, so `fn` sees a fresh credential and never writes.
	 */
	async modify(
		providerId: string,
		fn: (current: PiCredential | undefined) => Promise<PiCredential | undefined>,
	): Promise<PiCredential | undefined> {
		const refreshed = this.#source.refresh(providerId).then((credential) => {
			if (credential?.type === "oauth" && credential.expires - Date.now() <= PI_OAUTH_MIN_VALIDITY_MS) {
				throw new Error(`the credential source returned a ${providerId} token that expires within five minutes`);
			}
			return toPiCredential(credential);
		});
		this.#read.set(providerId, refreshed);
		refreshed.catch(() => {
			if (this.#read.get(providerId) === refreshed) this.#read.delete(providerId);
		});
		const current = await refreshed;
		if ((await fn(current)) !== undefined) {
			throw new Error(`pi actor cannot store credentials for ${providerId}; the credential source owns them`);
		}
		return current;
	}

	async delete(providerId: string): Promise<void> {
		throw new Error(`pi actor cannot delete credentials for ${providerId}; the credential source owns them`);
	}
}

/** Pi's OAuth credential type needs a refresh token field. Pi never uses it, because `modify` refreshes through the source. */
function toPiCredential(credential: PiProviderCredential | undefined): PiCredential | undefined {
	if (!credential) return undefined;
	if (credential.type === "api_key") return credential;
	return { ...credential, refresh: "" };
}
