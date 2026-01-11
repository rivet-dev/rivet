import type { RegistryConfig } from "@/mod";

export function getInspectorUrl(runConfig: RegistryConfig | undefined) {
	const url = new URL("https://inspect.rivet.dev");

	const overrideDefaultEndpoint = runConfig?.inspector?.defaultEndpoint;
	if (overrideDefaultEndpoint) {
		url.searchParams.set("u", overrideDefaultEndpoint);
	}

	return url.href;
}
