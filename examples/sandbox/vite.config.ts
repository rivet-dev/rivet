import react from "@vitejs/plugin-react";
import { defineConfig, type Plugin } from "vite";
import srvx from "vite-plugin-srvx";
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
	plugins: [react(), sqlRawPlugin(), ...srvx({ entry: "src/server.ts" })],
});
