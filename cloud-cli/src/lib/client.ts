/**
 * Lightweight typed client for the Rivet Cloud REST API (https://cloud-api.rivet.dev).
 *
 * All requests use the RIVET_CLOUD_TOKEN bearer token. We make direct fetch
 * calls rather than the auto-generated SDK so the CLI has zero heavy
 * framework dependencies and works under any Bun version.
 */

export interface InspectResponse {
	project: string;
	organization: string;
}

export interface Namespace {
	id: string;
	name: string;
	displayName: string;
	createdAt: string;
}

export interface ManagedPool {
	name: string;
	config: {
		image: {
			repository: string;
			tag: string;
		} | null;
		minCount: number;
		maxCount: number;
		environment?: Record<string, string>;
	};
}

export interface DockerCredentials {
	registryUrl: string;
	username: string;
	password: string;
}

export class CloudApiError extends Error {
	constructor(
		public readonly status: number,
		public readonly body: unknown,
		message: string,
	) {
		super(message);
		this.name = "CloudApiError";
	}
}

export class CloudClient {
	readonly baseUrl: string;
	private readonly token: string;

	constructor(opts: { token: string; baseUrl?: string }) {
		this.token = opts.token;
		this.baseUrl = (opts.baseUrl ?? "https://cloud-api.rivet.dev").replace(/\/$/, "");
	}

	private async request<T>(
		method: string,
		path: string,
		body?: unknown,
		query?: Record<string, string>,
	): Promise<T> {
		let url = `${this.baseUrl}${path}`;
		if (query && Object.keys(query).length > 0) {
			const params = new URLSearchParams(query);
			url = `${url}?${params}`;
		}

		const resp = await fetch(url, {
			method,
			headers: {
				Authorization: `Bearer ${this.token}`,
				"Content-Type": "application/json",
			},
			...(body !== undefined ? { body: JSON.stringify(body) } : {}),
		});

		if (!resp.ok) {
			let errBody: unknown;
			try {
				errBody = await resp.json();
			} catch {
				errBody = await resp.text();
			}
			const message =
				typeof errBody === "object" &&
				errBody !== null &&
				"message" in errBody
					? String((errBody as Record<string, unknown>).message)
					: `HTTP ${resp.status} ${resp.statusText}`;
			throw new CloudApiError(resp.status, errBody, message);
		}

		if (resp.status === 204) return undefined as unknown as T;
		return resp.json() as Promise<T>;
	}

	/** Inspect the current API token to get org + project. */
	async inspect(): Promise<InspectResponse> {
		return this.request<InspectResponse>("GET", "/api-tokens/inspect");
	}

	/** List namespaces in a project. */
	async listNamespaces(project: string, org: string): Promise<Namespace[]> {
		const data = await this.request<{ namespaces: Namespace[] }>(
			"GET",
			`/projects/${encodeURIComponent(project)}/namespaces`,
			undefined,
			{ org },
		);
		return data.namespaces;
	}

	/** Get a specific namespace. Returns null if 404. */
	async getNamespace(
		project: string,
		namespace: string,
		org: string,
	): Promise<Namespace | null> {
		try {
			const data = await this.request<{ namespace: Namespace }>(
				"GET",
				`/projects/${encodeURIComponent(project)}/namespaces/${encodeURIComponent(namespace)}`,
				undefined,
				{ org },
			);
			return data.namespace;
		} catch (err) {
			if (err instanceof CloudApiError && err.status === 404) return null;
			throw err;
		}
	}

	/** Create a namespace and return it. */
	async createNamespace(
		project: string,
		displayName: string,
		org: string,
	): Promise<Namespace> {
		const data = await this.request<{ namespace: Namespace }>(
			"POST",
			`/projects/${encodeURIComponent(project)}/namespaces`,
			{ displayName, org },
		);
		return data.namespace;
	}

	/** Get the managed pool, returning null if it does not exist yet. */
	async getManagedPool(
		project: string,
		namespace: string,
		pool: string,
		org: string,
	): Promise<ManagedPool | null> {
		try {
			const data = await this.request<{ managedPool: ManagedPool }>(
				"GET",
				`/projects/${encodeURIComponent(project)}/namespaces/${encodeURIComponent(namespace)}/managed-pools/${encodeURIComponent(pool)}`,
				undefined,
				{ org },
			);
			return data.managedPool;
		} catch (err) {
			if (err instanceof CloudApiError && err.status === 404) return null;
			throw err;
		}
	}

	/** Create or update the managed pool configuration. */
	async upsertManagedPool(
		project: string,
		namespace: string,
		pool: string,
		request: {
			org: string;
			image?: { repository: string; tag: string };
			minCount?: number;
			maxCount?: number;
			environment?: Record<string, string>;
			command?: string;
			args?: string;
		},
	): Promise<void> {
		await this.request<void>(
			"PUT",
			`/projects/${encodeURIComponent(project)}/namespaces/${encodeURIComponent(namespace)}/managed-pools/${encodeURIComponent(pool)}`,
			request,
		);
	}

	/** Retrieve Docker registry credentials for pushing images. */
	async getDockerCredentials(project: string, org: string): Promise<DockerCredentials> {
		const data = await this.request<DockerCredentials>(
			"POST",
			`/projects/${encodeURIComponent(project)}/docker/credentials`,
			undefined,
			{ org },
		);
		return data;
	}

	/**
	 * Open an SSE log stream. Returns an async iterator of log lines.
	 * The caller is responsible for calling `controller.abort()` when done.
	 */
	async *streamLogs(
		project: string,
		namespace: string,
		pool: string,
		opts: {
			contains?: string;
			region?: string;
			signal?: AbortSignal;
		} = {},
	): AsyncGenerator<{ timestamp: string; region: string; message: string }> {
		const query: Record<string, string> = {};
		if (opts.contains) query.contains = opts.contains;
		if (opts.region) query.region = opts.region;

		let url = `${this.baseUrl}/projects/${encodeURIComponent(project)}/namespaces/${encodeURIComponent(namespace)}/managed-pools/${encodeURIComponent(pool)}/logs`;
		if (Object.keys(query).length > 0) {
			url = `${url}?${new URLSearchParams(query)}`;
		}

		const resp = await fetch(url, {
			headers: {
				Authorization: `Bearer ${this.token}`,
				Accept: "text/event-stream",
			},
			signal: opts.signal,
		});

		if (!resp.ok) {
			throw new CloudApiError(
				resp.status,
				null,
				`Failed to open log stream: HTTP ${resp.status} ${resp.statusText}`,
			);
		}

		if (!resp.body) throw new Error("No response body for log stream");

		const reader = resp.body.getReader();
		const decoder = new TextDecoder();
		let buffer = "";

		try {
			while (true) {
				const { done, value } = await reader.read();
				if (done) break;
				buffer += decoder.decode(value, { stream: true });

				const events = buffer.split("\n\n");
				buffer = events.pop() ?? "";

				for (const eventBlock of events) {
					if (!eventBlock.trim()) continue;

					let eventType = "message";
					let dataStr = "";

					for (const line of eventBlock.split("\n")) {
						if (line.startsWith("event: ")) {
							eventType = line.slice(7).trim();
						} else if (line.startsWith("data: ")) {
							dataStr = line.slice(6).trim();
						}
					}

					if (eventType === "log" && dataStr) {
						try {
							const parsed = JSON.parse(dataStr);
							const entry = parsed.data ?? parsed;
							if (
								typeof entry === "object" &&
								entry !== null &&
								"timestamp" in entry &&
								"region" in entry &&
								"message" in entry
							) {
								yield entry as { timestamp: string; region: string; message: string };
							}
						} catch {
							yield { timestamp: new Date().toISOString(), region: "unknown", message: dataStr };
						}
					} else if (eventType === "end") {
						return;
					} else if (eventType === "error" && dataStr) {
						let errMessage = "Log stream error";
						try {
							const err = JSON.parse(dataStr);
							errMessage = (err as Record<string, unknown>).message as string ?? errMessage;
						} catch {
							errMessage = dataStr;
						}
						throw new Error(errMessage);
					}
				}
			}
		} finally {
			reader.releaseLock();
		}
	}
}
