import { MCP_DOCS_URL } from "@/components/mcp/client-tabs";

const RIVET_DOCS_URL = "https://rivet.dev/docs";

/** Sensitive clipboard text: it embeds the cluster admin token. */
export function serializeClusterAgentInstructions({
	cluster,
	mcpUrl,
	publicEndpoint,
	privateEndpoint,
	adminToken,
}: {
	cluster: string;
	mcpUrl: string;
	publicEndpoint: string | undefined;
	privateEndpoint: string | undefined;
	adminToken: string | undefined;
}): string | null {
	if (!publicEndpoint || !adminToken) return null;
	return [
		`Help me build on my Rivet cluster \`${cluster}\`. Use Rivet for every operation on it.`,
		"",
		"## 1. Connect the Rivet MCP server first",
		"Before anything else, add this remote MCP server to the agent you are running in, using that agent's own way of registering MCP servers:",
		"- Name: rivet",
		"- Transport: Streamable HTTP",
		`- URL: ${mcpUrl}`,
		`Per-client setup instructions are at ${MCP_DOCS_URL}.`,
		"The server authorizes against my Rivet account through a browser sign-in, so ask me to approve that step; you cannot complete it for me.",
		"If the connection is declined or fails, continue without it and say that MCP was skipped.",
		"",
		"## 2. Use Rivet for all operations",
		"Run every operation on this cluster through Rivet: the Rivet MCP tools first, then the Rivet CLI and SDKs. Use them to create and inspect actors, read state, pull logs, and deploy. Do not hand-roll HTTP calls against the cluster or reach for other infrastructure when Rivet covers the task.",
		`Read the docs when you need details: ${RIVET_DOCS_URL}`,
		"",
		"## 3. Configure the connection",
		`Set RIVET_ENDPOINT to the cluster's public endpoint: ${publicEndpoint}`,
		...(privateEndpoint
			? [
					`Workers running inside the cluster's network should use the internal endpoint instead: ${privateEndpoint}`,
				]
			: []),
		"Set RIVET_TOKEN to this admin token (decode the JSON string below):",
		JSON.stringify(adminToken),
		"The admin token has full access to the cluster. Use it only in server-side code and deploy pipelines. Store it in the project's secret manager or an untracked env file, never in source control, client bundles, command arguments, or logs. Do not repeat it in your response.",
		"",
		"## 4. Verify",
		"Start the app, create an actor, and call an action against the cluster, then confirm the actor through the Rivet MCP tools. If any check cannot be completed, report it as unverified rather than claiming success.",
	].join("\n");
}
