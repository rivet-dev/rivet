import { FreestyleSandboxes } from "freestyle-sandboxes";

// Validate required API keys at startup
const freestyleApiKey = process.env.FREESTYLE_API_KEY;
const anthropicApiKey = process.env.ANTHROPIC_API_KEY;

if (!freestyleApiKey) {
	throw new Error("FREESTYLE_API_KEY environment variable is required");
}

if (!anthropicApiKey) {
	throw new Error("ANTHROPIC_API_KEY environment variable is required");
}

export const freestyle = new FreestyleSandboxes({ apiKey: freestyleApiKey });

export async function requestDevServer({ repoId }: { repoId: string }) {
	const result = await freestyle.requestDevServer({
		repoId,
	});

	return {
		ephemeralUrl: result.ephemeralUrl,
		mcpEphemeralUrl: result.mcpEphemeralUrl,
		fs: result.fs,
		devCommandRunning: result.devCommandRunning,
		installCommandRunning: result.installCommandRunning,
		codeServerUrl: result.codeServerUrl,
		consoleUrl: `${result.ephemeralUrl}/__console`,
	};
}
