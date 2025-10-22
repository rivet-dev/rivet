// Cloud API client for fetching autofill data
import type {
	CloudBootstrapResponse,
	CloudNamespace,
	CloudNamespaceToken,
} from "./types";

const CLOUD_API_URL =
	process.env.NEXT_PUBLIC_CLOUD_API_URL || "https://cloud.rivet.dev";

class CloudApiClient {
	private baseUrl: string;

	constructor(baseUrl: string = CLOUD_API_URL) {
		this.baseUrl = baseUrl;
	}

	async bootstrap(): Promise<CloudBootstrapResponse> {
		const response = await fetch(`${this.baseUrl}/cloud/bootstrap`, {
			credentials: "include", // Include cookies for auth
		});

		if (!response.ok) {
			throw new Error(
				`Failed to fetch bootstrap data: ${response.statusText}`,
			);
		}

		return response.json();
	}

	async getNamespaces(projectId: string): Promise<CloudNamespace[]> {
		const response = await fetch(
			`${this.baseUrl}/cloud/projects/${projectId}/namespaces`,
			{
				credentials: "include",
			},
		);

		if (!response.ok) {
			throw new Error(
				`Failed to fetch namespaces: ${response.statusText}`,
			);
		}

		return response.json();
	}

	async getNamespaceToken(
		projectId: string,
		namespaceId: string,
	): Promise<CloudNamespaceToken> {
		const response = await fetch(
			`${this.baseUrl}/cloud/projects/${projectId}/namespaces/${namespaceId}/token`,
			{
				credentials: "include",
			},
		);

		if (!response.ok) {
			throw new Error(
				`Failed to fetch namespace token: ${response.statusText}`,
			);
		}

		return response.json();
	}
}

export const cloudApi = new CloudApiClient();
