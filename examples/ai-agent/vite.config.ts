import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
	plugins: [react()],
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
