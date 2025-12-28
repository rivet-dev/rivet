import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import dts from "vite-plugin-dts";

const __dirname = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
	plugins: [dts({ include: ["src"] })],
	build: {
		lib: {
			entry: resolve(__dirname, "src/mod.ts"),
			fileName: "mod",
			formats: ["cjs", "es"],
		},
	},
});
