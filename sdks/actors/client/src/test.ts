import { Client } from "./mod.ts";

/**
 * Uses the Rivet CLI to read the manager endpoint to connect to. This allows
 * for writing tests that run locally without hardcoding the manager endpoint.
 */
export async function readEndpointFromCli(): Promise<string> {
	// Read endpoint
	const cliPath = Deno.env.get("RIVET_CLI_PATH") ?? "rivet";
	const output = await new Deno.Command(cliPath, {
		args: ["manager", "endpoint"],
	}).output();
	if (!output.success) {
		throw new Error(
			`Read endpoint failed with ${output.code}:\n${new TextDecoder().decode(
				output.stderr,
			)}`,
		);
	}

	// Decode output
	const endpoint = new TextDecoder().decode(output.stdout).trim();
	return endpoint;
}

export class TestClient extends Client {
	public constructor() {
		super(readEndpointFromCli());
	}
}
