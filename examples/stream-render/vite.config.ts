import path from "node:path";
import { fileURLToPath } from "node:url";
import react from "@vitejs/plugin-react";
import { defineConfig, loadEnv } from "vite";

const managerPort = Number(process.env.RIVET_MANAGER_PORT) || 6420;
const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig(({ mode }) => {
	const env = loadEnv(mode, process.cwd(), "");
	const rivetPublic =
		env.VITE_RIVET_PUBLIC_ENDPOINT || env.RIVET_PUBLIC_ENDPOINT || "";

	return {
		plugins: [react()],
		publicDir: false,
		resolve: {
			alias: {
				shiki: path.resolve(__dirname, "frontend/shiki-stub.ts"),
			},
		},
		define: {
			"import.meta.env.VITE_RIVET_PUBLIC_ENDPOINT":
				JSON.stringify(rivetPublic),
		},
		build: {
			outDir: "public",
			emptyOutDir: true,
			chunkSizeWarningLimit: 1600,
			rollupOptions: {
				output: {
					manualChunks(id) {
						if (
							id.includes("node_modules/react/") ||
							id.includes("node_modules/react-dom")
						) {
							return "react-vendor";
						}
						if (id.includes("node_modules/@rivetkit")) {
							return "rivetkit-vendor";
						}
						if (id.includes("node_modules/render-dds")) {
							return "ui-vendor";
						}
					},
				},
			},
		},
		server: {
			clearScreen: false,
			proxy: {
				"/actors": {
					target: `http://127.0.0.1:${managerPort}`,
					ws: true,
				},
				"/metadata": { target: `http://127.0.0.1:${managerPort}` },
				"/health": { target: `http://127.0.0.1:${managerPort}` },
			},
		},
	};
});
