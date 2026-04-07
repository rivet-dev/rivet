import type { AppRegistry } from "../../src/rivet/registry.ts";

export type { AppRegistry };

/**
 * Rivet client base URL.
 * - **Dev**: local manager (Vite proxy).
 * - **Prod + Rivet Cloud**: `RIVET_PUBLIC_ENDPOINT` / `VITE_RIVET_PUBLIC_ENDPOINT` (pk URL), baked at build time.
 * - **Prod fallback**: same origin (embedded manager).
 */
export function rivetClientBase(): string {
	if (import.meta.env.DEV) return "http://localhost:6420";
	const fromBuild = import.meta.env.VITE_RIVET_PUBLIC_ENDPOINT as
		| string
		| undefined;
	if (fromBuild && fromBuild.length > 0) {
		return fromBuild.replace(/\/$/, "");
	}
	return window.location.origin;
}
