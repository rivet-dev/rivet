import { RIVET_GLOBAL_API_HOSTNAME } from "./rivet-constants";

/**
 * RivetKit uses `RIVET_ENVOY_VERSION` for runner versioning and deploy drains
 * ([docs](https://rivet.dev/docs/actors/versions)). On Render, `RENDER_GIT_COMMIT` is
 * injected at build and runtime ([Render default env](https://render.com/docs/environment-variables)),
 * so we derive a stable integer from the commit SHA—no manual bump per deploy.
 *
 * Override with `RIVET_ENVOY_VERSION` (or legacy `RIVET_RUNNER_VERSION`) when needed.
 */
function ensureRivetEnvoyVersionFromEnvironment(): void {
	if (
		process.env.RIVET_ENVOY_VERSION !== undefined &&
		process.env.RIVET_ENVOY_VERSION !== ""
	) {
		return;
	}
	const legacy = process.env.RIVET_RUNNER_VERSION;
	if (legacy !== undefined && legacy !== "") {
		process.env.RIVET_ENVOY_VERSION = legacy;
		return;
	}
	const sha = process.env.RENDER_GIT_COMMIT;
	if (sha && /^[0-9a-f]{7,40}$/i.test(sha)) {
		const n = Number.parseInt(sha.slice(0, 8), 16);
		process.env.RIVET_ENVOY_VERSION = String(n > 0 ? n : 1);
	}
}

ensureRivetEnvoyVersionFromEnvironment();

export const devPort = Number(process.env.RIVET_MANAGER_PORT) || 6420;
export const port = Number(process.env.PORT) || devPort;

export const publicStaticDir = process.env.PUBLIC_STATIC_DIR?.trim() || "public";

/** Rivet Cloud: serverless handler on this origin. */
export const useRivetCloud =
	process.env.NODE_ENV === "production" && Boolean(process.env.RIVET_ENDPOINT);

function rivetUrlUsesGlobalApiHost(urlStr: string | undefined): boolean {
	if (!urlStr) return false;
	try {
		return new URL(urlStr).hostname === RIVET_GLOBAL_API_HOSTNAME;
	} catch {
		return false;
	}
}

/** Warn if `RIVET_*` URLs use bare `api.rivet.dev` (runner requires regional host). */
export function warnIfRivetUsesGlobalHost(): void {
	if (!useRivetCloud) return;
	const ep = process.env.RIVET_ENDPOINT;
	const pub = process.env.RIVET_PUBLIC_ENDPOINT;
	if (rivetUrlUsesGlobalApiHost(ep)) {
		console.warn(
			"[rivet] RIVET_ENDPOINT uses api.rivet.dev. Replace it with the regional api-*.rivet.dev URL from dashboard.rivet.dev — otherwise the runner disconnects (must_use_regional_host).",
		);
	}
	if (rivetUrlUsesGlobalApiHost(pub)) {
		console.warn(
			"[rivet] RIVET_PUBLIC_ENDPOINT uses api.rivet.dev. Use the same regional host as your sk URL so the client matches your namespace.",
		);
	}
}
