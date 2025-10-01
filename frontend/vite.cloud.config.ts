import { defineConfig, loadEnv, mergeConfig } from "vite";
import { cloudEnvSchema } from "./src/lib/env";
import engineConfig from "./vite.engine.config";

// https://vitejs.dev/config/
export default defineConfig((config) => {
	const env = loadEnv(config.mode, process.cwd(), "");
	cloudEnvSchema.parse(env);
	return mergeConfig(
		engineConfig(config),
		defineConfig({
			base: "/",
			define: {
				__APP_TYPE__: JSON.stringify("cloud"),
			},
			server: {
				port: 43710,
				proxy: {},
			},
			preview: {
				port: 43710,
			},
		}),
	);
});
