import type { registry } from "../../src/actors.ts";

export type AppRegistry = typeof registry;

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
