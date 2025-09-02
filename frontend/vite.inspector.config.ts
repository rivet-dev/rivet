import { defineConfig, mergeConfig } from "vite";
import engineConfig from "./vite.engine.config";

// https://vitejs.dev/config/
export default defineConfig((args) => {
	return mergeConfig(
		engineConfig(args),
		defineConfig({
			base: "/",
			define: {
				__APP_TYPE__: JSON.stringify("inspector"),
			},
			server: {
				port: 43709,
				proxy: {},
			},
			preview: {
				port: 43709,
			},
		}),
	);
});
