/**
 * Derive `RIVET_ENVOY_VERSION` from Render's `RENDER_GIT_COMMIT` so deploys
 * get a unique version automatically — no manual bump needed.
 */
function ensureRivetEnvoyVersion(): void {
	if (process.env.RIVET_ENVOY_VERSION) return;

	if (process.env.RIVET_RUNNER_VERSION) {
		process.env.RIVET_ENVOY_VERSION = process.env.RIVET_RUNNER_VERSION;
		return;
	}

	const sha = process.env.RENDER_GIT_COMMIT;
	if (sha && /^[0-9a-f]{7,40}$/i.test(sha)) {
		const n = Number.parseInt(sha.slice(0, 8), 16);
		process.env.RIVET_ENVOY_VERSION = String(n > 0 ? n : 1);
	}
}

ensureRivetEnvoyVersion();

export const port = Number(process.env.PORT) || 6420;

export const useRivetCloud =
	process.env.NODE_ENV === "production" &&
	Boolean(process.env.RIVET_ENDPOINT);
