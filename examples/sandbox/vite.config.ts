import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";
import { readFileSync } from "node:fs";

function sqlRawPlugin(): Plugin {
	return {
		name: "sql-raw",
		transform(_code, id) {
			if (id.endsWith(".sql")) {
				const content = readFileSync(id, "utf-8");
				return { code: `export default ${JSON.stringify(content)};` };
			}
		},
	};
}

export default defineConfig({
	plugins: [react(), sqlRawPlugin()],
	publicDir: false,
	build: {
		outDir: "public",
		emptyOutDir: true,
	},
	server: {
		clearScreen: false,
		proxy: {
			"/actors": { target: "http://localhost:6420", ws: true },
			"/metadata": { target: "http://localhost:6420" },
			"/health": { target: "http://localhost:6420" },
		},
	},
});
