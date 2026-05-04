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
	server: {
		proxy: {
			"/api/rivet": {
				target: "http://127.0.0.1:3000",
				changeOrigin: true,
				ws: true,
			},
		},
	},
});
