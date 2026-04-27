import type { RegistryConfig } from "@/registry/config";

type NativeBindings = typeof import("@rivetkit/rivetkit-napi");

export interface RunnerConfigDatacenterRequest {
	normal?: Record<string, unknown>;
	serverless?: {
		url: string;
		headers: Record<string, string>;
		maxRunners: number;
		minRunners: number;
		requestLifespan: number;
		runnersMargin: number;
		slotsPerRunner: number;
		metadataPollInterval?: number;
	};
	metadata?: Record<string, unknown>;
	drainOnVersionUpgrade?: boolean;
}

async function loadNativeBindings(): Promise<NativeBindings> {
	return import(["@rivetkit", "rivetkit-napi"].join("/"));
}

function engineAdminConfig(config: RegistryConfig) {
	if (!config.endpoint) {
		throw new Error("endpoint is required for runner config updates");
	}

	return {
		endpoint: config.endpoint,
		token: config.token,
		namespace: config.namespace,
		headers: config.headers,
	};
}

export async function upsertRunnerConfigForAllDatacenters(
	config: RegistryConfig,
	runnerName: string,
	datacenterConfig: RunnerConfigDatacenterRequest,
): Promise<void> {
	const bindings = await loadNativeBindings();
	await bindings.upsertRunnerConfigForAllDatacenters(
		engineAdminConfig(config),
		runnerName,
		datacenterConfig,
	);
}
