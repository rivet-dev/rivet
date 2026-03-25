/**
 * Authentication helpers.
 *
 * The CLI primarily uses RIVET_CLOUD_TOKEN (a static API token) — the same
 * secret used by the rivet-dev/deploy-action. This token is passed to the
 * Cloud API as a Bearer token.
 *
 * For interactive browser-based Clerk auth flows, the pattern would mirror
 * what the frontend does (getToken from a Clerk session). We expose the
 * low-level helper here so commands can easily call it.
 */

import { colors } from "../utils/output.ts";

export function resolveToken(cliToken: string | undefined): string {
	const token = cliToken ?? process.env.RIVET_CLOUD_TOKEN;
	if (!token) {
		console.error(
			colors.error(
				"No token found. Provide RIVET_CLOUD_TOKEN env var or pass --token <token>.",
			),
		);
		console.error(
			colors.dim(
				"  Get your token from https://hub.rivet.dev → project → Connect → Rivet Cloud",
			),
		);
		process.exit(1);
	}
	return token;
}
