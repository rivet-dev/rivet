/// <reference types="vite/client" />

declare const __APP_BUILD_ID__: string;

declare module "vite-plugin-favicons-inject" {
	import type { Plugin } from "vite";

	export default function vitePluginFaviconsInject(
		iconPath: string,
		// biome-ignore lint/suspicious/noExplicitAny: <explanation>
		options: Record<string, any>,
	): Plugin;
}

declare module "*?shiki&lang=bash" {
	const src: string;
	export default src;
	export const source: string;
}
