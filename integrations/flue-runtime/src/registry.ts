export interface FlueRivetRegistry {
	startAndWait(): Promise<void>;
	parseConfig(): {
		endpoint?: string;
		namespace: string;
		token?: string;
		headers?: Record<string, string>;
		envoy: { poolName: string };
	};
}

let installedRegistry: FlueRivetRegistry | undefined;

/** The registry created by the generated Rivet target entrypoint. */
export const flueRegistry: FlueRivetRegistry = {
	startAndWait() {
		return requireRegistry().startAndWait();
	},
	parseConfig() {
		return requireRegistry().parseConfig();
	},
};

/** @internal Installs the generated target registry for imported agent modules. */
export function installFlueRegistry(registry: FlueRivetRegistry): void {
	installedRegistry = registry;
}

function requireRegistry(): FlueRivetRegistry {
	if (!installedRegistry) {
		throw new Error(
			'[flue] The Rivet target registry is not installed. Use the agent through the generated Flue server.',
		);
	}
	return installedRegistry;
}
