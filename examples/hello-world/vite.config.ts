import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
	plugins: [react()],
	publicDir: false,
	build: {
		outDir: "public",
		emptyOutDir: true,
	},
	server: {
		// Disable screen clearing so concurrently output stays readable
		clearScreen: false,
		proxy: {
			// Forward manager API and WebSocket requests to the backend
			"/actors": { target: "http://localhost:6420", ws: true },
			"/metadata": { target: "http://localhost:6420" },
			"/health": { target: "http://localhost:6420" },
		},
	},
});
