import * as crypto from "node:crypto";
import path from "node:path";
import type { UserConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";

// Shared vite config used by all frontend builds (engine, cloud, inspector, ladle).
export function baseViteConfig(): UserConfig {
	return {
		plugins: [tsconfigPaths()],
		define: {
			__APP_TYPE__: JSON.stringify(process.env.APP_TYPE || "engine"),
			__APP_BUILD_ID__: JSON.stringify(
				`${new Date().toISOString()}@${crypto.randomUUID()}`,
			),
		},
		resolve: {
			alias: {
				"@": path.resolve(__dirname, "./src"),
			},
		},
		optimizeDeps: {
			include: ["@fortawesome/*", "@rivet-gg/icons", "@rivet-gg/cloud"],
		},
		worker: {
			format: "es",
		},
	};
}
